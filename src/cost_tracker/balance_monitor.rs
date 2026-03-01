use solana_sdk::{pubkey::Pubkey, signature::{Keypair, Signer}};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::time::Duration;
use tracing::{info, warn, error};
use anyhow::Result;
use crate::rpc::RpcManager;

pub struct BalanceMonitor {
    rpc_manager: Arc<RpcManager>,
    wallet_pubkey: Pubkey,
    min_balance_lamports: u64,
    trading_paused: Arc<AtomicBool>,
    last_balance: Arc<std::sync::Mutex<Option<u64>>>,
}

impl BalanceMonitor {
    pub fn new(rpc_manager: Arc<RpcManager>, wallet: &Keypair, min_balance_sol: f64) -> Self {
        let min_balance_lamports = (min_balance_sol * 1_000_000_000.0) as u64;
        
        Self {
            rpc_manager,
            wallet_pubkey: wallet.pubkey(),min_balance_lamports,
            trading_paused: Arc::new(AtomicBool::new(false)),
            last_balance: Arc::new(std::sync::Mutex::new(None)),
        }
    }
    
    pub fn start_monitoring(self: Arc<Self>) {
        tokio::spawn(async move {
            self.monitor_loop().await;
        });
    }
    
    async fn monitor_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        
        loop {
            interval.tick().await;
            
            match self.check_balance().await {
                Ok(balance) => {
                    let balance_sol = balance as f64 / 1_000_000_000.0;
                    let paused = self.trading_paused.load(Ordering::Relaxed);
                    
                    info!(
                        "Wallet balance: {} SOL ({}) - Trading {}",
                        balance_sol,
                        if balance < self.min_balance_lamports {
                            "BELOW THRESHOLD".to_string()
                        } else {
                            "OK".to_string()
                        },
                        if paused { "PAUSED" } else { "ACTIVE" }
                    );
                    
                    // Update last balance
                    if let Ok(mut last) = self.last_balance.lock() {
                        *last = Some(balance);
                    }
                }
                Err(e) => {
                    error!("Failed to check balance: {}", e);
                }
            }
        }
    }
    
    pub async fn check_balance(&self) -> Result<u64> {
        let balance = self.rpc_manager.get_balance(&self.wallet_pubkey).await?;
        
        let should_pause = balance < self.min_balance_lamports;
        let was_paused = self.trading_paused.load(Ordering::Relaxed);
        
        if should_pause && !was_paused {
            warn!(
                "Balance {} below threshold {}, PAUSING TRADING",
                balance, self.min_balance_lamports
            );
            self.trading_paused.store(true, Ordering::Relaxed);
        } else if !should_pause && was_paused {
            info!(
                "Balance {} recovered above threshold {}, RESUMING TRADING",
                balance, self.min_balance_lamports
            );
            self.trading_paused.store(false, Ordering::Relaxed);
        }
        
        Ok(balance)
    }
    
    pub fn is_trading_paused(&self) -> bool {
        self.trading_paused.load(Ordering::Relaxed)
    }
    
    pub fn get_estimated_trade_cost(&self) -> u64 {
        // Base fee + priority fee + Jito tip
        5000 + 10000 + 100000 // ~0.000115 SOL
    }
    
    pub fn can_afford_trade(&self) -> bool {
        if let Ok(last) = self.last_balance.lock() {
            if let Some(balance) = *last {
                let cost = self.get_estimated_trade_cost();
                let remaining_after = balance.saturating_sub(cost);
                return remaining_after > self.min_balance_lamports;
            }
        }
        false // If we don't know balance, assume we can't afford
    }
    
    pub async fn get_balance_sol(&self) -> Result<f64> {
        let balance = self.rpc_manager.get_balance(&self.wallet_pubkey).await?;
        Ok(balance as f64 / 1_000_000_000.0)
    }
}

impl Clone for BalanceMonitor {
    fn clone(&self) -> Self {
        Self {
            rpc_manager: self.rpc_manager.clone(),
            wallet_pubkey: self.wallet_pubkey,
            min_balance_lamports: self.min_balance_lamports,
            trading_paused: self.trading_paused.clone(),
            last_balance: self.last_balance.clone(),
        }
    }
}
