use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use crate::grpc::SubscribeUpdate;
use serde_json::Value;

/// Process gRPC updates from the Yellowstone/Richat stream.
/// - `rx` receives raw textual updates (JSON) produced by the Richat client.
/// - `alert_tx` is an optional channel used to forward brief alerts to the Telegram forwarder.
/// - `program_filter` contains program IDs (e.g. Pump.fun, Raydium) to watch for in transactions.
pub async fn process_grpc_updates(
    mut rx: UnboundedReceiver<SubscribeUpdate>,
    alert_tx: Option<UnboundedSender<String>>,
    program_filter: Vec<String>,
) {
    let mut processed: u64 = 0;
    let mut detected: u64 = 0;

    while let Some(update) = rx.recv().await {
        processed += 1;
        // Attempt to parse JSON payload heuristically. Richat/Yellowstone messages can be
        // complex protobuf-turned-JSON objects; we apply lightweight pattern matching.
        match serde_json::from_str::<Value>(&update) {
            Ok(v) => {
                let mut is_relevant = false;

                // Detect new token account creations: presence of `account` with `mint` field
                if let Some(account) = v.get("account") {
                    if account.get("mint").is_some() {
                        is_relevant = true;
                        detected += 1;
                        let mint = account.get("mint").and_then(|m| m.as_str()).unwrap_or("<unknown>");
                        info!(mint = %mint, "Detected new token account (possible new mint)");
                        if let Some(tx) = &alert_tx {
                            let _ = tx.send(format!("New token account detected: mint={} ", mint));
                        }
                    }
                    // Balance updates may appear as `lamports` or `balance`
                    if account.get("lamports").is_some() || account.get("balance").is_some() {
                        is_relevant = true;
                        detected += 1;
                        let owner = account.get("owner").and_then(|o| o.as_str()).unwrap_or("<unknown>");
                        let lamports = account.get("lamports").and_then(|l| l.as_u64()).unwrap_or(0);
                        info!(owner = %owner, lamports = lamports, "Balance change detected for account owner");
                        if let Some(tx) = &alert_tx {
                            let _ = tx.send(format!("Balance update: owner={} lamports={} ", owner, lamports));
                        }
                    }
                }

                // Detect transactions involving specific program IDs. Typical messages may include
                // a `transaction` object or `instructions` array with `programId` entries.
                if let Some(tx_obj) = v.get("transaction").or_else(|| v.get("tx")) {
                    if let Some(instructions) = tx_obj.get("instructions").or_else(|| tx_obj.get("instr")) {
                        if instructions.is_array() {
                            for instr in instructions.as_array().unwrap().iter() {
                                if let Some(prog) = instr.get("programId") {
                                    if let Some(pid) = prog.as_str() {
                                        if program_filter.iter().any(|p| p == pid) {
                                            is_relevant = true;
                                            detected += 1;
                                            info!(program = %pid, "Detected transaction involving watched program");
                                            if let Some(tx) = &alert_tx {
                                                let _ = tx.send(format!("Transaction involving watched program: {}", pid));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // If not relevant, apply basic filtering to skip verbose logs
                if !is_relevant {
                    // Skip logging every non-relevant update; only count it
                }
            }
            Err(e) => {
                // Could not parse JSON; log at debug level and skip
                debug!(error = ?e, "Failed to parse gRPC update as JSON; raw=\"{}\"", update);
            }
        }

        // Periodic metrics/logging
        if processed % 100 == 0 {
            info!(processed = processed, detected = detected, "gRPC update processing metrics");
        }
    }

    info!(processed = processed, detected = detected, "gRPC update stream closed and metrics");
}
pub mod market_cap;
pub mod swap;
pub mod jito;
pub mod transaction_builder;

use crate::config::settings::Settings;
use tokio::sync::RwLock;
use crate::rpc::RpcManager;
use crate::telegram::TelegramBot;
use crate::cost_tracker::{CostTracker, BalanceMonitor};
use crate::trading::transaction_builder::TransactionBuilder;
use crate::trading::market_cap::MarketCapCache;
use crate::trading::swap::JupiterApi;
use crate::trading::jito::JitoClient;
use crate::grpc::stream_handler::NewTokenEvent;
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use tokio::sync::Mutex;
use std::str::FromStr;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
struct Position {
    mint: Pubkey,
    entry_price: f64, // SOL per token
}

pub struct TradeManager {
    rpc: Arc<RpcManager>,
    tx_builder: Arc<TransactionBuilder>,
    config: Arc<RwLock<Settings>>,
    telegram: Arc<TelegramBot>,
    cost_tracker: Arc<CostTracker>,
    market_cap_cache: Arc<MarketCapCache>,
    jupiter: Arc<JupiterApi>,
    jito: Arc<JitoClient>,
    balance_monitor: Arc<BalanceMonitor>,
    traded: Mutex<HashSet<String>>,
    positions: Mutex<HashMap<String, Position>>,
}

impl TradeManager {
    pub fn new(
        rpc: Arc<RpcManager>,
        tx_builder: Arc<TransactionBuilder>,
        config: Arc<RwLock<Settings>>,
        telegram: Arc<TelegramBot>,
        cost_tracker: Arc<CostTracker>,
        balance_monitor: Arc<BalanceMonitor>,
        market_cap_cache: Arc<MarketCapCache>,
        jupiter: Arc<JupiterApi>,
        jito: Arc<JitoClient>,
    ) -> Self {
        TradeManager {
            rpc,
            tx_builder,
            config,
            telegram,
            cost_tracker,
            market_cap_cache,
            jupiter,
            jito,
            balance_monitor,
            traded: Mutex::new(HashSet::new()),
            positions: Mutex::new(HashMap::new()),
        }
    }

    pub async fn handle_new_token(&self, event: NewTokenEvent) -> Result<Option<String>> {
        let mint_str = event.mint.to_string();

        // already traded?
        {
            let traded = self.traded.lock().await;
            if traded.contains(&mint_str) {
                debug!("mint {} already traded, skipping", mint_str);
                return Ok(None);
            }
        }

        let cfg = self.config.read().await;
        // filter by market cap
        if let Some(cap) = self.market_cap_cache.get(&mint_str).await? {
            if cap < cfg.min_market_cap || cap > cfg.max_market_cap {
                debug!("mint {} market cap {} outside range", mint_str, cap);
                return Ok(None);
            }
        } else {
            debug!("could not find market cap for {}", mint_str);
            return Ok(None);
        }

        // ask balance monitor again just before buy
        if !self.balance_monitor.can_afford_trade() {
            warn!("balance monitor says cannot afford trade, skipping {}", mint_str);
            let _ = self.telegram.send_alert("⚠️ skipped buy because balance too low").await;
            return Ok(None);
        }

        let tx_id = self.execute_buy(event.mint).await?;

        // record that we purchased this mint
        {
            let mut traded = self.traded.lock().await;
            traded.insert(mint_str.clone());
        }

        Ok(Some(tx_id))
    }

    pub async fn monitor_positions(&self) -> Result<()> {
        let mut to_close = Vec::new();
        {
            let positions = self.positions.lock().await;
            for (mint_str, pos) in positions.iter() {
                let cfg = self.config.read().await;
            if let Ok(quote) = self
                    .jupiter
                    .quote(mint_str, 1, cfg.slippage_bps)
                    .await
                {
                    let current_price = quote.input_amount as f64 / quote.output_amount as f64;
                    let change = (current_price - pos.entry_price) / pos.entry_price * 100.0;
                    if change >= cfg.take_profit_percent
                        || change <= -cfg.stop_loss_percent
                    {
                        to_close.push(mint_str.clone());
                    }
                }
            }
        }

        for mint_str in to_close {
            let mint = Pubkey::from_str(&mint_str)?;
            let sale_id = self.execute_sell(mint).await?;
            self.positions.lock().await.remove(&mint_str);
            let _ = self
                .telegram
                .send_alert(&format!(
                    "💵 position {} closed (tx {})",
                    mint_str, sale_id
                ))
                .await;
        }

        Ok(())
    }

    async fn execute_buy(&self, mint: Pubkey) -> Result<String> {
        let cfg = self.config.read().await;
        let amount_lamports = (cfg.buy_amount_sol * 1_000_000_000.0) as u64;
        let quote = self
            .jupiter
            .quote(&mint.to_string(), amount_lamports, cfg.slippage_bps)
            .await?;
        let entry_price = quote.input_amount as f64 / quote.output_amount as f64;

        // ask Jupiter for swap instructions (SOL -> token)
        let instructions = self
            .jupiter
            .swap_instructions(&mint.to_string(), amount_lamports, cfg.slippage_bps)
            .await?;
        let (recent_hash, _) = self.rpc.get_recent_blockhash().await?;
        let tx = self
            .tx_builder
            .build_transaction(instructions, recent_hash)?;
        let serialized = bincode::serialize(&tx)?;
        let resp = self
            .jito
            .send_bundle(&serialized, cfg.jito_tip_lamports as f64 / 1_000_000_000.0)
            .await?;

        // store position
        {
            let mut positions = self.positions.lock().await;
            positions.insert(
                mint.to_string(),
                Position { mint, entry_price },
            );
        }

        // add ledger entry
        self.cost_tracker
            .add_entry(mint.to_string(), cfg.buy_amount_sol, 0.0);
        // refresh balance monitor cache
        let _ = self.balance_monitor.check_balance().await;

        let _ = self
            .telegram
            .send_alert(&format!(
                "🛒 bought {} @ {:.9} SOL (approx)",
                mint, entry_price
            ))
            .await;
        Ok(resp)
    }

    async fn execute_sell(&self, mint: Pubkey) -> Result<String> {
        // sell 1 token for SOL at market rate
let cfg = self.config.read().await;
        let quote = self
            .jupiter
            .quote(&mint.to_string(), 1, cfg.slippage_bps)
            .await?;
        let exit_price = quote.output_amount as f64 / quote.input_amount as f64;

        let instructions = self
            .jupiter
            .swap_instructions(&mint.to_string(), 1, cfg.slippage_bps)
            .await?; // sell one token
        let (recent_hash, _) = self.rpc.get_recent_blockhash().await?;
        let tx = self
            .tx_builder
            .build_transaction(instructions, recent_hash)?;
        let serialized = bincode::serialize(&tx)?;
        let resp = self
            .jito
            .send_bundle(&serialized, cfg.jito_tip_lamports as f64 / 1_000_000_000.0)
            .await?;

        let _ = self
            .telegram
            .send_alert(&format!(
                "💵 sold {} @ {:.9} SOL (approx)",
                mint, exit_price
            ))
            .await;
        Ok(resp)
    }

    // helpers for tests
    #[cfg(test)]
    pub async fn mark_traded(&self, mint: &str) {
        let mut t = self.traded.lock().await;
        t.insert(mint.to_string());
    }

    #[cfg(test)]
    pub async fn is_traded(&self, mint: &str) -> bool {
        let t = self.traded.lock().await;
        t.contains(mint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Keypair;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn traded_skip_logic() {
        let rpc = Arc::new(RpcManager::new(vec![]).await.unwrap());
        let tx_builder = Arc::new(TransactionBuilder::new(Keypair::new()));
        let config = Settings::default();
        let telegram = Arc::new(
            TelegramBot::new("dummy", Arc::new(RwLock::new(config.clone())))
                .await
                .unwrap(),
        );
        let cost_tracker = Arc::new(CostTracker::new());
        let balance_monitor = Arc::new(
            crate::cost_tracker::balance_monitor::BalanceMonitor::new(
                rpc.clone(),
                &Keypair::new(),
                0.01,
            ),
        );
        let market_cap_cache = Arc::new(MarketCapCache::new());
        let jupiter = Arc::new(JupiterApi::new());
        let jito = Arc::new(JitoClient::new(String::new(), None));
        let manager = TradeManager::new(
            rpc,
            tx_builder,
            config.clone(),
            telegram,
            cost_tracker,
            balance_monitor,
            market_cap_cache,
            jupiter,
            jito,
        );

        let mint = "So11111111111111111111111111111111111111112";
        manager.mark_traded(mint).await;
        let event = NewTokenEvent {
            mint: Pubkey::from_str(mint).unwrap(),
            dev_wallet: Pubkey::default(),
            slot: 0,
            tx_signature: "".to_string(),
        };
        let res = manager.handle_new_token(event).await.unwrap();
        assert!(res.is_none());
    }

    #[test]
    fn price_change_calculation() {
        let entry = 1.0;
        let current = 1.6;
        let change = (current - entry) / entry * 100.0;
        assert_eq!(change, 60.0);
    }
}
