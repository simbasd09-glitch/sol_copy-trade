use crate::config::BotConfig;
use crate::rpc_pool::RpcPool;
use crate::telegram::TelegramClient;
use dashmap::DashMap;
use std::sync::Arc;
use log::{info, warn};

#[derive(Clone, Debug)]
pub enum Event {
    ProgramCreate { mint: String, dev: String },
    Buy { trader: String, mint: String, sol: f64, token_amount: f64 },
    Sell { trader: String, mint: String, sol: f64, token_amount: f64 },
}

#[derive(Clone)]
pub struct Trader {
    cfg: BotConfig,
    positions: Arc<DashMap<String, Position>>,
    rpc_pool: Arc<RpcPool>,
    tg: Arc<TelegramClient>,
    mint_by_dev: Arc<DashMap<String, String>>,
}

#[derive(Clone, Debug)]
pub struct Position {
    pub mint: String,
    pub entry_price: f64,
    pub token_amount: f64,
    pub highest_price: f64,
}

impl Trader {
    pub fn new(
        cfg: BotConfig,
        positions: Arc<DashMap<String, Position>>,
        rpc_pool: Arc<RpcPool>,
        tg: Arc<TelegramClient>,
        mint_by_dev: Arc<DashMap<String, String>>,
    ) -> Arc<Self> {
        Arc::new(Self { cfg, positions, rpc_pool, tg, mint_by_dev })
    }

    pub async fn handle_event(self: Arc<Self>, evt: Event) {
        match evt {
            Event::ProgramCreate { mint, dev } => {
                info!("Detected create: mint={} dev={}", mint, dev);
                self.mint_by_dev.insert(dev.clone(), mint.clone());
                let _ = self.tg.send_message(&format!("Detected new mint {} by dev {}", mint, dev)).await;
            }
            Event::Buy { trader, mint, sol, token_amount } => {
                info!("Buy event trader={} mint={} sol={} tokens={}", trader, mint, sol, token_amount);
                // If trader equals dev for this mint, emit CopySignal
                if let Some(dev_mint) = self.mint_by_dev.get(&trader) {
                    if dev_mint.value() == &mint {
                        info!("Dev trade detected for mint {} by dev {}", mint, trader);
                        self.on_copy_signal(true, trader, mint, sol, token_amount).await;
                    }
                }
            }
            Event::Sell { trader, mint, sol, token_amount } => {
                info!("Sell event trader={} mint={} sol={} tokens={}", trader, mint, sol, token_amount);
                if let Some(dev_mint) = self.mint_by_dev.get(&trader) {
                    if dev_mint.value() == &mint {
                        info!("Dev sell detected for mint {} by dev {} -> copying sell", mint, trader);
                        self.on_copy_signal(false, trader, mint, sol, token_amount).await;
                    }
                }
            }
        }
    }

    async fn on_copy_signal(&self, is_buy: bool, dev: String, mint: String, dev_sol: f64, dev_tokens: f64) {
        if !self.cfg.copy_trade.enabled {
            return;
        }

        if is_buy && !self.cfg.copy_trade.copy_buys {
            return;
        }
        if !is_buy && !self.cfg.copy_trade.copy_sells {
            return;
        }

        // Convert USD cap to SOL using config.sol_price_usd
        let max_sol = self.cfg.copy_trade.max_usd_per_trade / self.cfg.sol_price_usd;
        let mut buy_sol = dev_sol * self.cfg.copy_trade.multiplier;
        if buy_sol > max_sol { buy_sol = max_sol; }
        if buy_sol < self.cfg.copy_trade.min_dev_trade_sol {
            info!("Ignoring tiny copy trade: {} SOL < min", buy_sol);
            return;
        }

        // Market cap filter placeholder: in real implementation query token marketcap
        if self.cfg.entry_filter.use_market_cap {
            // TODO: query marketcap; skip if out of range
        }

        // Build transaction placeholder: in real code construct, sign and serialize tx
        let fake_tx = base64::encode(format!("{}:{}:{}", if is_buy {"buy"} else {"sell"}, mint, buy_sol));
        match self.rpc_pool.send_transaction(&fake_tx).await {
            Ok(sig) => {
                info!("Submitted copy {} tx sig={}", if is_buy {"buy"} else {"sell"}, sig);
                let _ = self.tg.send_message(&format!("Submitted copy {} for {}: sig={}", if is_buy {"BUY"} else {"SELL"}, mint, sig)).await;
                // On buy, record position
                if is_buy {
                    let pos = Position { mint: mint.clone(), entry_price: buy_sol / dev_tokens.max(1.0), token_amount: dev_tokens * self.cfg.copy_trade.multiplier, highest_price: buy_sol / dev_tokens.max(1.0) };
                    self.positions.insert(mint.clone(), pos);
                } else {
                    // On sell, remove position if present
                    self.positions.remove(&mint);
                }
            }
            Err(e) => warn!("Failed to submit tx: {}", e),
        }
    }
}
