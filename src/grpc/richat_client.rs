use std::env;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use dotenvy::dotenv;
use futures::{StreamExt, TryStreamExt};
use parking_lot::RwLock;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time::sleep;
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, error, info, warn};

use crate::grpc::SubscribeUpdate;

// NOTE: The exact SDK types/names may vary across solana-stream-sdk releases.
// This implementation follows the common "GeyserGrpcClient" pattern used by
// the SDK: create a tonic Channel, build the client, call a subscribe method
// and stream updates. Replace concrete request/response types if they differ
// in your installed SDK.

/// Publicly exposed spawn function. Returns an UnboundedReceiver of textual updates.
pub fn spawn_richat_stream(endpoint: Option<String>, x_token: Option<String>, program_filter: Vec<String>) -> UnboundedReceiver<SubscribeUpdate> {
    let (tx, rx) = mpsc::unbounded_channel::<SubscribeUpdate>();

    // Run background task
    tokio::spawn(async move {
        // endpoint and token come from caller (main): prefer those values
        let endpoint = endpoint.clone().unwrap_or_default();
        if endpoint.is_empty() {
            error!("no gRPC endpoint provided; richat stream will not start");
            return;
        }
        let token = x_token.clone();

        // Shared last-seen slot to resume after reconnects
        let last_slot: Arc<RwLock<Option<u64>>> = Arc::new(RwLock::new(None));
        debug!(?program_filter, "using program filter for subscription");

        let mut backoff_secs = 1u64;
        loop {
            info!(endpoint = %endpoint, "connecting to Richat gRPC endpoint");

            match connect_and_stream(&endpoint, token.as_deref(), last_slot.clone(), tx.clone()).await {
                Ok(()) => {
                    info!("richat stream ended cleanly, restarting connection");
                    backoff_secs = 1;
                }
                Err(e) => {
                    warn!(error = ?e, "richat stream error, will reconnect with backoff");
                    // exponential backoff
                    sleep(Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(64);
                }
            }

            // if receiver closed, stop the task
            if tx.is_closed() {
                info!("richat consumer dropped receiver; exiting richat task");
                return;
            }
        }
    });

    rx
}

async fn connect_and_stream(
    endpoint: &str,
    token: Option<&str>,
    last_slot: Arc<RwLock<Option<u64>>>,
    tx: UnboundedSender<SubscribeUpdate>,
) -> Result<()> {
    // Build tonic Endpoint and configure HTTP/2 keepalive to send ping/pong every 10s
    let mut ep = Endpoint::from_shared(endpoint.to_string())?;
    // keepalive settings: ping every 10s; set timeout shorter than 30s to avoid server disconnect
    ep = ep
        .http2_keep_alive_interval(Some(Duration::from_secs(10)))
        .http2_keep_alive_timeout(Some(Duration::from_secs(5)));

    // If the SDK exposes builder helpers for TLS etc, integrate them here. For now create a Channel.
    // If a token is supplied, we will attach it later as a header/interceptor when building the SDK client.
    let channel = ep.connect().await?;

    // Create SDK client using the GeyserGrpcClient pattern.
    // Replace the path below with the actual client type from your installed solana-stream-sdk.
    // Example placeholder:
    // let mut client = solana_stream_sdk::geyser::GeyserGrpcClient::new(channel);

    // Build subscribe request; attempt to resume from last_slot if present
    let from_slot = *last_slot.read();
    debug!(?from_slot, "starting subscription, resuming from");

    // The following is a generic pattern: call client.subscribe(request).await? and get a tonic::Streaming
    // We adapt the stream to yield String updates and update last_slot as messages arrive.

    // NOTE: The concrete types below are placeholders: `SubscribeRequest` and `GeyserGrpcClient`.
    // If your SDK uses different names, replace them accordingly.
    #[allow(unused_mut, unused_variables, dead_code)]
    {
        // Example pseudocode using SDK types:
        // let req = solana_stream_sdk::proto::SubscribeRequest { from_slot }; 
        // let mut stream = client.subscribe(req).await?.into_inner();

        // For safety if SDK shapes differ, implement a minimal manual ping-read loop using tonic raw streaming if needed.
    }

    // For now implement a fallback loop that keeps the connection alive and reports status.
    // Replace this with real streaming logic wired to the SDK's stream when available.
    info!("richat connected, entering read loop");

    loop {
        // Simulate receiving an update; the real implementation should pull from the SDK stream.
        // Here we sleep briefly and then emit a heartbeat update so downstream can be exercised.
        tokio::select! {
            _ = sleep(Duration::from_secs(10)) => {
                let update = format!("richat-heartbeat: slot={}", last_slot.read().unwrap_or(0));
                if tx.send(update).is_err() {
                    info!("consumer dropped; exiting stream task");
                    return Ok(());
                }
            }
        }
    }
}
