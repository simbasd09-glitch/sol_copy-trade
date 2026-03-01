use crate::config::BotConfig;
use crate::rpc_pool::RpcPool;
use crate::telegram::TelegramClient;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use log::info;

use crate::trader::Position;

pub struct PositionManager {
    cfg: BotConfig,
    positions: Arc<DashMap<String, Position>>,
    rpc_pool: Arc<RpcPool>,
    tg: Arc<TelegramClient>,
}

impl PositionManager {
    pub fn new(cfg: BotConfig, positions: Arc<DashMap<String, Position>>, rpc_pool: Arc<RpcPool>, tg: Arc<TelegramClient>) -> Self {
        Self { cfg, positions, rpc_pool, tg }
    }

    pub async fn run(self) {
        loop {
            // Iterate positions and evaluate exits
            let keys: Vec<String> = self.positions.iter().map(|kv| kv.key().clone()).collect();
            for mint in keys {
                if let Some(mut entry) = self.positions.get_mut(&mint) {
                    // Placeholder: update current price by querying an oracle or DEX
                    let current_price = entry.entry_price * 1.02; // pretend price moved up 2%
                    if current_price > entry.highest_price {
                        entry.highest_price = current_price;
                    }

                    // Compute pct change from entry
                    let pct = (current_price - entry.entry_price) / entry.entry_price * 100.0;

                    // Take profit
                    if pct >= self.cfg.exit.take_profit_percent {
                        info!("Take profit triggered for {} pct={}", mint, pct);
                        let fake_tx = base64::encode(format!("sell:{}", mint));
                        let _ = self.rpc_pool.send_transaction(&fake_tx).await;
                        let _ = self.tg.send_message(&format!("Take profit sell for {} at pct {}", mint, pct)).await;
                        self.positions.remove(&mint);
                        continue;
                    }

                    // Trailing stop logic (simplified)
                    let stop_price = if entry.highest_price > entry.entry_price * (1.0 + self.cfg.exit.stop_loss_breakeven_after_percent / 100.0) {
                        // trail stop percent below highest
                        entry.highest_price * (1.0 - self.cfg.exit.stop_loss_trail_percent / 100.0)
                    } else {
                        // initial stop at entry*(1 - initial percent)
                        entry.entry_price * (1.0 - self.cfg.exit.stop_loss_initial_percent / 100.0)
                    };

                    if current_price <= stop_price {
                        info!("Stop loss triggered for {} current={} stop={}", mint, current_price, stop_price);
                        let fake_tx = base64::encode(format!("sell:{}", mint));
                        let _ = self.rpc_pool.send_transaction(&fake_tx).await;
                        let _ = self.tg.send_message(&format!("Stop loss sell for {} at price {}", mint, current_price)).await;
                        self.positions.remove(&mint);
                    }
                }
            }

            sleep(Duration::from_secs(10)).await;
        }
    }
}
