use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::CommitmentConfig;
use solana_sdk::{
    signature::Signature,
    transaction::Transaction,
    hash::Hash,
    pubkey::Pubkey,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
    collections::HashMap,
};

pub mod hedged_client;
pub use hedged_client::HedgedClient;

use tokio::sync::Mutex;
use anyhow::{Result, anyhow};
use tracing::{info, warn, error, debug};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
pub struct ProviderConfig {
    pub id: String,
    pub url: String,
    pub weight: u32,
}

struct ProviderHealth {
    last_success: Instant,
    consecutive_failures: u32,
    avg_latency: Duration,
    last_check: Instant,
}

impl ProviderHealth {
    fn new() -> Self {
        Self {
            last_success: Instant::now(),
            consecutive_failures: 0,
            avg_latency: Duration::from_millis(100),
            last_check: Instant::now(),
        }
    }
    
    fn update(&mut self, latency: Duration, success: bool) {
        self.last_check = Instant::now();
        if success {
            self.last_success = Instant::now();
            self.consecutive_failures = 0;
            // Exponential moving average
            self.avg_latency = Duration::from_nanos(
                ((self.avg_latency.as_nanos() * 9 + latency.as_nanos()) / 10) as u64
            );
        } else {
            self.consecutive_failures += 1;
        }
    }
    
    fn is_dead(&self) -> bool {
        self.consecutive_failures > 5 || 
        self.last_success.elapsed() > Duration::from_secs(60)
    }
}

pub struct RpcManager {
    clients: Arc<Vec<Arc<RpcClient>>>,
    providers: Vec<ProviderConfig>,
    fastest_index: Arc<AtomicUsize>,
    health_map: Arc<Mutex<HashMap<String, ProviderHealth>>>,
}

impl RpcManager {
    pub async fn new(providers: Vec<ProviderConfig>) -> Result<Self> {
        let mut clients = Vec::new();
        let mut health_map = HashMap::new();
        
        for provider in &providers {
            let client = Arc::new(RpcClient::new_with_commitment(
                provider.url.clone(),
                CommitmentConfig::confirmed(),
            ));
            // Test connection
            match client.get_version().await {
                Ok(version) => {
                    info!("Connected to {}: version {}", provider.id, version.solana_core);
                    clients.push(client.clone());
                    health_map.insert(provider.id.clone(), ProviderHealth::new());
                }
                Err(e) => {
                    warn!("Failed to connect to {}: {}", provider.id, e);
                    // Still push but mark as dead initially
                    clients.push(client.clone());
                    let mut health = ProviderHealth::new();
                    health.consecutive_failures = 6; // Mark as dead
                    health_map.insert(provider.id.clone(), health);
                }
            }
        }
        
        let manager = Self {
            clients: Arc::new(clients),
            providers,
            fastest_index: Arc::new(AtomicUsize::new(0)),
            health_map: Arc::new(Mutex::new(health_map)),
        };
        
        // Start health check loop
        let manager_clone = manager.clone();tokio::spawn(async move {
            manager_clone.health_check_loop().await;
        });
        
        Ok(manager)
    }
    
    pub async fn get_fastest_client(&self) -> Result<Arc<RpcClient>> {
        let index = self.fastest_index.load(Ordering::Relaxed);
        if index < self.clients.len() {
            Ok(self.clients[index].clone())
        } else {
            Err(anyhow!("No healthy RPC clients available"))
        }
    }
    
    pub async fn get_recent_blockhash(&self) -> Result<(Hash, u64)> {
        // Race all clients for fastest response
        let mut futures = Vec::new();
        
        for (i, client) in self.clients.iter().enumerate() {
            let client = client.clone();
            futures.push(tokio::spawn(async move {
                let start = Instant::now();
                match client.get_latest_blockhash().await {
                    Ok(hash) => {
                        let latency = start.elapsed();
                        Ok((i, hash, latency))
                    }
                    Err(e) => Err((i, e)),
                }
            }));
        }
        
        let mut last_error = None;
        for future in futures {
            match future.await {
                Ok(Ok((i, hash, latency))) => {
                    // Update health for this provider
                    let mut health_map = self.health_map.lock().await;
                    if let Some(health) = health_map.get_mut(&self.providers[i].id) {
                        health.update(latency, true);
                    }
                    return Ok((hash, latency.as_micros() as u64));
                }
                Ok(Err((i, e))) => {
                    last_error = Some(e);
                    let mut health_map = self.health_map.lock().await;
                    if let Some(health) = health_map.get_mut(&self.providers[i].id) {
                        health.update(Duration::from_secs(1), false);
                    }
                }
                Err(e) => {
                    error!("Health check task panicked: {}", e);
                }
            }
        }
        
        Err(anyhow!("All RPC providers failed: {:?}", last_error))
    }
    
    pub async fn send_transaction(&self, tx: &Transaction) -> Result<Signature> {
        let start = Instant::now();
        let mut last_error = None;
        
        // Try fastest provider first
        if let Ok(client) = self.get_fastest_client().await {
            match client.send_transaction(tx).await {
                Ok(sig) => {
                    debug!("Transaction sent via fastest provider in {:?}", start.elapsed());
                    return Ok(sig);
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }
        
        // Fall back to racing all providers
        let mut futures = Vec::new();
        for client in self.clients.iter() {
            let client = client.clone();
            let tx = tx.clone();
            futures.push(tokio::spawn(async move {
                client.send_transaction(&tx).await
            }));
        }
        
        for future in futures {
            match future.await {
                Ok(Ok(sig)) => {
                    return Ok(sig);
                }
                Ok(Err(e)) => {
                    last_error = Some(e);
                }
                Err(e) => {
                    error!("Send transaction task panicked: {}", e);
                }
            }
        }
        
        Err(anyhow!("All providers failed to send transaction: {:?}", last_error))
    }
    
    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64> {if let Ok(client) = self.get_fastest_client().await {
            return Ok(client.get_balance(pubkey).await?);
        }
        
        // Try all clients
        for client in self.clients.iter() {
            if let Ok(balance) = client.get_balance(pubkey).await {
                return Ok(balance);
            }
        }
        
        Err(anyhow!("All providers failed to get balance"))
    }
    
    async fn health_check_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            let mut latencies = Vec::new();
            let mut health_map = self.health_map.lock().await;
            
            for (i, client) in self.clients.iter().enumerate() {
                let provider_id = &self.providers[i].id;
                let health = health_map.entry(provider_id.clone()).or_insert_with(ProviderHealth::new);
                
                let start = Instant::now();
                match client.get_version().await {
                    Ok(_) => {
                        let latency = start.elapsed();
                        health.update(latency, true);
                        latencies.push((i, latency));
                        debug!("Provider {} latency: {:?}", provider_id, latency);
                    }
                    Err(e) => {
                        warn!("Health check failed for {}: {}", provider_id, e);
                        health.update(Duration::from_secs(1), false);
                    }
                }
            }
            
            // Update fastest provider (only consider healthy ones)
            if !latencies.is_empty() {
                latencies.sort_by_key(|(_, latency)| *latency);
                if let Some((fastest_idx, _)) = latencies.first() {
                    let old = self.fastest_index.load(Ordering::Relaxed);
                    self.fastest_index.store(*fastest_idx, Ordering::Relaxed);
                    if old != *fastest_idx {
                        info!("Fastest provider changed to {} (latency: {:?})", 
                              self.providers[*fastest_idx].id, latencies[0].1);
                    }
                }
            }
            
            // Log dead providers
            for (provider_id, health) in health_map.iter() {
                if health.is_dead() {
                    warn!("Provider {} is dead", provider_id);
                }
            }
        }
    }
}

impl Clone for RpcManager {
    fn clone(&self) -> Self {
        Self {
            clients: self.clients.clone(),
            providers: self.providers.clone(),
            fastest_index: self.fastest_index.clone(),
            health_map: self.health_map.clone(),
        }
    }
}
