use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{sleep, Duration};
use tracing::info;

pub mod stream_handler;
pub mod richat_client;

/// Lightweight placeholder for `SubscribeUpdate` until a real gRPC proto crate is added.
pub type SubscribeUpdate = String;

/// Spawn a placeholder gRPC stream task that emits textual updates.
/// This avoids depending on a specific proto crate; replace with a real implementation
/// that uses your generated prost/tonic types when available.
pub fn spawn_grpc_stream(endpoint: String, _program_filter: Vec<String>) -> UnboundedReceiver<SubscribeUpdate> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<SubscribeUpdate>();

    tokio::spawn(async move {
        info!("(stub) connecting to gRPC endpoint {}", endpoint);
        let mut backoff = 1u64;
        loop {
            // stub behavior: send a heartbeat string occasionally so downstream code can be exercised
            if tx.send(format!("stub-update: connected to {}", endpoint)).is_err() {
                info!("gRPC consumer dropped receiver, exiting stub stream");
                return;
            }
            sleep(Duration::from_secs(10 * backoff)).await;
            backoff = (backoff * 2).min(6);
        }
    });

    rx
}
