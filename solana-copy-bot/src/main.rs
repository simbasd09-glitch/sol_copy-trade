use std::sync::Arc;

mod config;
mod helius;
mod position_manager;
mod rpc_pool;
mod telegram;
mod trader;

use anyhow::Result;
use config::BotConfig;
use dashmap::DashMap;
use env_logger::Env;
use helius::HeliusSubscriber;
use position_manager::PositionManager;
use rpc_pool::RpcPool;
use telegram::TelegramClient;
use trader::Trader;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cfg = BotConfig::from_file("config.yaml").expect("failed to load config.yaml");

    let shared_mint_by_dev: Arc<DashMap<String, String>> = Arc::new(DashMap::new());
    let positions = Arc::new(DashMap::new());

    let rpc_pool = Arc::new(RpcPool::new(cfg.clone()));
    let helius = HeliusSubscriber::new(cfg.clone());
    let tg = TelegramClient::new(cfg.clone());

    // Position manager handles exits and trailing stops
    let pm = PositionManager::new(cfg.clone(), positions.clone(), rpc_pool.clone(), tg.clone());
    tokio::spawn(async move { pm.run().await });

    // Trader handles copy logic and submissions
    let trader = Trader::new(cfg.clone(), positions.clone(), rpc_pool.clone(), tg.clone(), shared_mint_by_dev.clone());

    // Helius subscriber (mock or real) pushes events to trader
    helius.subscribe(move |evt| {
        let trader = trader.clone();
        tokio::spawn(async move { trader.handle_event(evt).await });
    })
    .await;

    // block until Ctrl+C so the spawned tasks can run
    tokio::signal::ctrl_c().await.expect("failed to install ctrl+c handler");

    Ok(())
}
