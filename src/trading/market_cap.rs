use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
struct DexScreenerResponse {
    pairs: Vec<DexPair>,
}

#[derive(Debug, Deserialize)]
struct DexPair {
    marketCap: Option<u64>,
    liquidity: Option<f64>,
    symbol: Option<String>,
}

pub struct MarketCapCache {
    inner: Mutex<HashMap<String, (u64, Instant)>>,
    client: Client,
}

impl MarketCapCache {
    pub fn new() -> Self {
        MarketCapCache {
            inner: Mutex::new(HashMap::new()),
            client: Client::new(),
        }
    }

    pub async fn get(&self, mint: &str) -> Result<Option<u64>> {
        let now = Instant::now();
        if let Some((cap, ts)) = self.inner.lock().unwrap().get(mint) {
            if now.duration_since(*ts) < Duration::from_secs(30) {
                return Ok(Some(*cap));
            }
        }
        let url = format!("https://api.dexscreener.com/latest/dex/tokens/{}", mint);
        let resp: DexScreenerResponse = self.client.get(&url).send().await?.json().await?;
        if let Some(pair) = resp.pairs.get(0) {
            if let Some(cap) = pair.marketCap {
                self.inner
                    .lock()
                    .unwrap()
                    .insert(mint.to_string(), (cap, now));
                return Ok(Some(cap));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_cache() {
        let cache = MarketCapCache::new();
        let cap = cache.get("So11111111111111111111111111111111111111112").await;
        assert!(cap.is_ok());
    }

    #[tokio::test]
    async fn test_manual_insert() {
        let cache = MarketCapCache::new();
        // insert value directly (we are inside module so can access inner)
        let now = Instant::now();
        cache
            .inner
            .lock()
            .unwrap()
            .insert("testmint".to_string(), (1234, now));
        let cap = cache.get("testmint").await.unwrap();
        assert_eq!(cap, Some(1234));
    }
}
