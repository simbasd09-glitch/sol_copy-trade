mod config;
mod rpc;
mod grpc;
mod grpc::richat_client;
mod trading;
mod health;
mod cost_tracker;
mod telegram;

use std::env;
use tokio::signal;
use tracing::{info, error};
use dotenv::dotenv;
use anyhow::Result;

use crate::config::Config;
use crate::rpc::HedgedClient;
use crate::telegram::TelegramBot;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

#[tokio::main]
async fn main() -> Result<()> {
    // load env and tracing
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting application");

    // load config file path from env or default
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".into());
    let cfg = match Config::from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to load config {}: {}", config_path, e);
            return Err(e);
        }
    };

    // build provider list, allow override via CHAINSTACK_URL
    let mut providers: Vec<(String, String)> = Vec::new();
    if let Ok(url) = env::var("CHAINSTACK_URL") {
        providers.push(("chainstack".to_string(), url));
    } else {
        for p in cfg.rpc_providers.iter() {
            providers.push((p.name.clone(), p.url.clone()));
        }
    }

    info!("Using {} RPC providers", providers.len());

    // initialize hedged RPC client
    let hedged = HedgedClient::new(providers);

    // periodically dump hedged-client health statistics
    let hedged_clone = hedged.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let stats = hedged_clone.get_stats();
            info!("hedged client stats: {:?}", stats);
        }
    });

    // alert channel for critical notifications
    let (alert_tx, alert_rx) = unbounded_channel::<String>();

    // initialize optional Telegram bot forwarder
    if let Some(bot) = TelegramBot::from_env() {
        bot.spawn_forwarder(alert_rx);
        info!("Telegram alerts enabled");
    } else {
        // no bot configured; drop receiver to avoid holding resources
        drop(alert_rx);
        info!("Telegram not configured (TELEGRAM_BOT_TOKEN/CHAT_ID missing)");
    }

    // spawn Richat gRPC stream and trading task if endpoint provided (env overrides config)
    let grpc_env_raw = env::var("GRPC_ENDPOINT").ok();
    // If env var is present but empty, treat as disabled per validation rule
    let grpc_env = grpc_env_raw.clone().filter(|s| !s.trim().is_empty());
    let endpoint_to_use = grpc_env.or_else(|| if !cfg.grpc.endpoint.trim().is_empty() { Some(cfg.grpc.endpoint.clone()) } else { None });

    // load token from env or config
    let token_env = env::var("X_TOKEN").ok();
    let token_to_use = token_env.or_else(|| cfg.grpc.x_token.clone());

    if let Some(endpoint) = endpoint_to_use {
        info!(endpoint = %endpoint, "using gRPC endpoint");
        let rx = crate::grpc::richat_client::spawn_richat_stream(Some(endpoint), token_to_use.clone(), cfg.grpc.program_filter.clone());
        let tx_clone: Option<UnboundedSender<String>> = Some(alert_tx.clone());
        let program_filter_clone = cfg.grpc.program_filter.clone();
        tokio::spawn(async move { trading::process_grpc_updates(rx, tx_clone, program_filter_clone).await });
    } else {
        // If env var `GRPC_ENDPOINT` was set but empty, explicitly log disabled
        if grpc_env_raw.is_some() && grpc_env_raw.unwrap().trim().is_empty() {
            warn!("GRPC_ENDPOINT environment variable is set but empty; gRPC disabled");
        }
        let _ = alert_tx.send("No gRPC endpoint configured, skipping gRPC stream".to_string());
        info!("No gRPC endpoint configured, skipping gRPC stream");
    }

    // start health server
    let shutdown_tx = health::spawn_health_server();

    // graceful shutdown on Ctrl+C
    signal::ctrl_c().await?;
    info!("Shutdown requested");
    let _ = shutdown_tx.send(());
    // give some time to shutdown gracefully
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    Ok(())
}
