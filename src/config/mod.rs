pub mod settings;

use serde::Deserialize;
use std::fs;

/// RPC provider configuration entries
#[derive(Debug, Deserialize, Clone)]
pub struct RpcProvider {
    pub name: String,
    pub url: String,
    pub role: String,
    pub weight: f64,
}

/// gRPC section of the config
#[derive(Debug, Deserialize, Clone)]
pub struct GrpcConfig {
    pub endpoint: String,
    pub program_filter: Vec<String>,
    #[serde(default)]
    pub x_token: Option<String>,
}

/// Trading parameters section of the config
#[derive(Debug, Deserialize, Clone)]
pub struct TradingConfig {
    pub min_liquidity_sol: f64,
    pub slippage_bps: u32,
    pub buy_amount_sol: f64,
}

/// Top-level configuration structure matching `config.toml` in project root.
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub network: String,
    pub rpc_providers: Vec<RpcProvider>,
    pub grpc: GrpcConfig,
    pub trading: TradingConfig,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&content)?;
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
network = "devnet"

[[rpc_providers]]
name = "chainstack"
url = "https://solana-devnet.chainstack.com/your-endpoint"
role = "all"
weight = 1.0

[[rpc_providers]]
name = "publicnode"
url = "https://devnet.solana-rpc.publicnode.com"
role = "all"
weight = 1.0

[grpc]
endpoint = "https://your-erpc-endpoint.grpc"
program_filter = ["6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"]
x_token = ""

[trading]
min_liquidity_sol = 0.1
slippage_bps = 500
buy_amount_sol = 0.01
"#;

    #[test]
    fn parse_sample() {
        let cfg: Config = toml::from_str(SAMPLE).expect("should parse sample");
        assert_eq!(cfg.network, "devnet");
        assert_eq!(cfg.rpc_providers.len(), 2);
        assert_eq!(cfg.trading.slippage_bps, 500);
    }
}
