use crate::config::BotConfig;
use log::info;
use std::sync::Arc;

#[derive(Clone)]
pub struct RpcPool {
    cfg: Arc<BotConfig>,
}

impl RpcPool {
    pub fn new(cfg: BotConfig) -> Self {
        Self { cfg: Arc::new(cfg) }
    }

    // Choose an RPC endpoint from the pool with simple round-robin or failover.
    pub async fn send_transaction(&self, _raw_tx_base64: &str) -> anyhow::Result<String> {
        // For demonstration we log and return a fake signature.
        // Replace with actual JSON-RPC `sendTransaction` calls and error handling.
        info!("rpc_pool: would send transaction (len={})", _raw_tx_base64.len());
        Ok("FakeSignature111111111111111111111111111111111111".to_string())
    }

    pub async fn helius_send(&self, endpoint: &str, payload: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let resp = client.post(endpoint).json(payload).send().await?;
        let json = resp.json().await?;
        Ok(json)
    }
}
