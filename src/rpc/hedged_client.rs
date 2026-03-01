use anyhow::{Result, anyhow};
use hedged_rpc_client::{HedgeConfig, ProviderConfig, ProviderId, HedgedRpcClient};
use solana_sdk::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use std::collections::VecDeque;
use tokio::sync::RwLock;
use tokio::time::sleep;

#[derive(Debug, Clone)]
struct ProviderHealth {
    pub last_response: Option<Instant>,
    pub window: VecDeque<bool>, // true=success, false=failure, last N entries
    pub avg_latency_ms: f64,
    pub disabled_until: Option<Instant>,
}

impl ProviderHealth {
    fn new() -> Self {
        Self { last_response: None, window: VecDeque::with_capacity(10), avg_latency_ms: 0.0, disabled_until: None }
    }

    fn record(&mut self, success: bool, latency_ms: f64) {
        self.last_response = Some(Instant::now());
        if self.window.len() == 10 {
            self.window.pop_front();
        }
        self.window.push_back(success);
        // simple moving average update
        if success {
            if self.avg_latency_ms == 0.0 {
                self.avg_latency_ms = latency_ms;
            } else {
                self.avg_latency_ms = (self.avg_latency_ms * 0.8) + (latency_ms * 0.2);
            }
        } else {
            // on failure, slightly increase avg_latency to deprioritize
            self.avg_latency_ms = if self.avg_latency_ms == 0.0 { 1000.0 } else { self.avg_latency_ms * 1.1 };
        }
    }

    fn error_rate(&self) -> f64 {
        if self.window.is_empty() { return 0.0; }
        let failures = self.window.iter().filter(|&&s| !s).count() as f64;
        failures / (self.window.len() as f64)
    }
}

/// A thin wrapper around `hedged_rpc_client::HedgedRpcClient` for this project.
#[derive(Clone)]
pub struct HedgedClient {
    // original provider list (owned)
    providers: Vec<(String, String)>,
    cfg: HedgeConfig,
    inner: Arc<RwLock<Arc<HedgedRpcClient>>>,
    health: Arc<DashMap<String, ProviderHealth>>,
}

impl HedgedClient {
    /// Creates a new hedged client from a list of `(name, url)` tuples.
    ///
    /// The provider names are coerced to `'static` by leaking the string; this is
    /// acceptable for long-lived process configuration.
    pub fn new(providers: Vec<(String, String)>) -> Self {
        // Keep owned provider list
        let mut provider_cfgs: Vec<ProviderConfig> = Vec::new();
        for (name, url) in &providers {
            let leaked = Box::leak(name.clone().into_boxed_str());
            provider_cfgs.push(ProviderConfig { id: ProviderId(leaked), url: url.clone() });
        }

        let len = provider_cfgs.len();
        let cfg = HedgeConfig {
            initial_providers: len,
            hedge_after: Duration::from_millis(10),
            max_providers: len,
            min_slot: None,
            overall_timeout: Duration::from_millis(500),
        };

        let client = HedgedRpcClient::new(provider_cfgs.clone(), cfg.clone());

        let inner = Arc::new(RwLock::new(Arc::new(client)));
        let health = Arc::new(DashMap::new());
        // populate health map
        for (name, _) in &providers {
            health.insert(name.clone(), ProviderHealth::new());
        }

        let hc = Self {
            providers: providers.clone(),
            cfg: cfg.clone(),
            inner: inner.clone(),
            health: health.clone(),
        };

        // spawn monitor task
        let providers_clone = providers.clone();
        let cfg_clone = cfg.clone();
        let inner_clone = inner.clone();
        let health_clone = health.clone();
        let handle = tokio::spawn(async move {
            // cooldown in seconds
            let cooldown = Duration::from_secs(60);
            loop {
                sleep(Duration::from_secs(30)).await;

                // evaluate providers
                let mut active: Vec<(String, String)> = Vec::new();
                for (name, url) in &providers_clone {
                    let mut keep = true;
                    if let Some(mut entry) = health_clone.get_mut(name) {
                        // if disabled until still in future, skip
                        if let Some(until) = entry.disabled_until {
                            if Instant::now() < until {
                                keep = false;
                            } else {
                                entry.disabled_until = None;
                            }
                        }

                        let err_rate = entry.error_rate();
                        if err_rate > 0.10 {
                            // disable for cooldown
                            entry.disabled_until = Some(Instant::now() + cooldown);
                            keep = false;
                        }
                    }
                    if keep {
                        active.push((name.clone(), url.clone()));
                    }
                }

                // reorder active based on avg latency (fastest first)
                active.sort_by(|a, b| {
                    let la = health_clone.get(&a.0).map(|e| e.avg_latency_ms).unwrap_or(0.0);
                    let lb = health_clone.get(&b.0).map(|e| e.avg_latency_ms).unwrap_or(0.0);
                    la.partial_cmp(&lb).unwrap_or(std::cmp::Ordering::Equal)
                });

                // rebuild inner HedgedRpcClient if change
                let provider_cfgs: Vec<ProviderConfig> = active.iter().map(|(name, url)| {
                    let leaked = Box::leak(name.clone().into_boxed_str());
                    ProviderConfig { id: ProviderId(leaked), url: url.clone() }
                }).collect();

                // create new client and swap
                if !provider_cfgs.is_empty() {
                    let new_client = HedgedRpcClient::new(provider_cfgs, cfg_clone.clone());
                    let mut guard = inner_clone.write().await;
                    *guard = Arc::new(new_client);
                }
            }
        });

        // spawn monitor task (detached)
        let _ = handle;
        hc
    }

    /// Returns the latest blockhash from whichever provider responded first.
    ///
    /// The returned `String` is the provider name (as passed into `new`).
    pub async fn get_latest_blockhash(&self) -> Result<(String, Hash)> {
        // perform hedged call using current inner client
        let guard = self.inner.read().await;
        let inner = guard.clone();

        // measure latency per provider indirectly via hedged client
        let start = Instant::now();
        // use the public API on HedgedRpcClient rather than calling its private hedged_call
        let res = inner.get_latest_blockhash().await;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        match res {
            Ok((provider_id, hash)) => {
                // record success
                if let Some(mut entry) = self.health.get_mut(&provider_id.0.to_string()) {
                    entry.record(true, elapsed_ms);
                }
                Ok((provider_id.0.to_string(), hash))
            }
            Err(e) => {
                // record failure for all providers involved (best effort)
                for key in self.health.iter().map(|kv| kv.key().clone()).collect::<Vec<_>>() {
                    if let Some(mut entry) = self.health.get_mut(&key) {
                        entry.record(false, elapsed_ms);
                    }
                }
                Err(anyhow!("hedged RPC error: {}", e))
            }
        }
    }

    /// Return a snapshot of provider health stats: (last_response_ms_ago, error_rate, avg_latency_ms)
    pub fn get_stats(&self) -> std::collections::HashMap<String, (Option<u128>, f64, f64)> {
        let mut out = std::collections::HashMap::new();
        for kv in self.health.iter() {
            let name = kv.key().clone();
            let h = kv.value();
            let last_ms = h.last_response.map(|t| Instant::now().duration_since(t).as_millis());
            let err = h.error_rate();
            let avg = h.avg_latency_ms;
            out.insert(name, (last_ms, err, avg));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn construct_and_fail() {
        // point at an obviously invalid URL so call returns error
        let client = HedgedClient::new(vec![("foo".to_string(), "http://127.0.0.1:0".to_string())]);
        let res = client.get_latest_blockhash().await;
        assert!(res.is_err());
    }
}
