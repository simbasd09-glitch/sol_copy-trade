//! Devnet event-driven test harness for Solana copy trading bot
//! - Monitors a target wallet using WebSocket (onLogs/onAccountChange)
//! - Measures latency and logs metrics in JSON

use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::{Keypair, Signer, read_keypair_file};
// Ensure Signer trait is in scope for pubkey()
use solana_sdk::signature::Signer;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::system_transaction;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::timeout;
use anyhow::Result;

const DEVNET_RPC: &str = "https://api.devnet.solana.com";
const DEVNET_WS: &str = "wss://api.devnet.solana.com";

#[derive(Default)]
struct Metrics {
    total_events: usize,
    total_latency_ms: u128,
    failed: usize,
    dropped: usize,
    retries: usize,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    // 1. Generate two wallets
    let wallet1 = Keypair::new();
    let wallet2 = Keypair::new();
    let pub1 = wallet1.pubkey();
    let pub2 = wallet2.pubkey();
    println!("Test wallets: {} {}", pub1, pub2);

    // 2. Airdrop SOL
    let rpc = RpcClient::new(DEVNET_RPC.to_string());
    rpc.request_airdrop(&pub1, 2_000_000_000).await?;
    rpc.request_airdrop(&pub2, 2_000_000_000).await?;
    tokio::time::sleep(Duration::from_secs(5)).await;

    // 3. Subscribe to logs for wallet1
    let metrics = Arc::new(Mutex::new(Metrics::default()));
    let metrics_clone = metrics.clone();
    let (ws, _unsub) = PubsubClient::logs_subscribe(
        DEVNET_WS,
        &pub1.to_string(),
        solana_client::rpc_config::RpcTransactionLogsConfig {
            commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
            ..Default::default()
        },
    ).await?;

    // 4. Simulate a transfer from wallet1 to wallet2
    let now = Instant::now();
    let tx = system_transaction::transfer(&wallet1, &pub2, 1_000_000_000, rpc.get_latest_blockhash().await?);
    let send_time = Instant::now();
    let sig = rpc.send_and_confirm_transaction(&tx).await?;
    let send_ms = send_time.elapsed().as_millis();

    // 5. Listen for event and measure detection time
    let mut detected = false;
    let mut ws_stream = ws;
    let mut detection_time = None;
    let mut confirmation_time = None;
    let mut retries = 0;
    let mut failed = false;
    let mut dropped = false;
    let mut log_json = json!({});

    let timeout_ms = 10_000;
    let event_start = Instant::now();
    while event_start.elapsed().as_millis() < timeout_ms {
        if let Ok(Some(log)) = timeout(Duration::from_millis(500), ws_stream.next()).await {
            if let Some(log) = log {
                let detect_ms = now.elapsed().as_millis();
                detection_time = Some(detect_ms);
                // Confirm transaction
                let conf_start = Instant::now();
                let _ = rpc.confirm_transaction(&sig).await;
                confirmation_time = Some(conf_start.elapsed().as_millis());
                detected = true;
                log_json = json!({
                    "event_detected_ms": detect_ms,
                    "tx_build_ms": send_ms,
                    "confirmation_ms": confirmation_time,
                    "total_latency_ms": detect_ms + confirmation_time.unwrap_or(0),
                    "sig": sig.to_string(),
                });
                break;
            }
        }
    }
    if !detected {
        dropped = true;
        failed = true;
    }
    // 6. Log metrics
    {
        let mut m = metrics_clone.lock().await;
        m.total_events += 1;
        m.total_latency_ms += log_json["total_latency_ms"].as_u64().unwrap_or(0) as u128;
        if failed { m.failed += 1; }
        if dropped { m.dropped += 1; }
        m.retries += retries;
    }
    println!("{}", serde_json::to_string_pretty(&log_json)?);
    Ok(())
}
