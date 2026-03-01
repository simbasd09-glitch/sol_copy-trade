use serde::Deserialize;
use std::fs;

#[derive(Clone, Debug, Deserialize)]
pub struct RpcEntry {
    pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HeliusConfig {
    pub url: String,
    pub grpc: String,
    pub auth_token: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct JitoConfig {
    pub bundle_url: String,
    pub tip_account: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Rpcs {
    pub primary: Vec<RpcEntry>,
    pub helius: HeliusConfig,
    pub jito: JitoConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CopyTradeConfig {
    pub enabled: bool,
    pub multiplier: f64,
    pub max_usd_per_trade: f64,
    pub copy_buys: bool,
    pub copy_sells: bool,
    pub min_dev_trade_sol: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EntryFilterConfig {
    pub use_market_cap: bool,
    pub min_market_cap_usd: f64,
    pub max_market_cap_usd: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExitConfig {
    pub take_profit_percent: f64,
    pub stop_loss_initial_percent: f64,
    pub stop_loss_breakeven_after_percent: f64,
    pub stop_loss_trail_percent: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BuyConfig {
    pub slippage_bps: u32,
    pub jito_tip_sol: f64,
    pub max_priority_fee_micro_lamports: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProgramsConfig {
    pub pump_fun: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token: String,
    pub chat_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub file: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BotConfig {
    pub sol_price_usd: f64,
    pub copy_trade: CopyTradeConfig,
    pub entry_filter: EntryFilterConfig,
    pub exit: ExitConfig,
    pub buy: BuyConfig,
    pub rpcs: Rpcs,
    pub programs: ProgramsConfig,
    pub telegram: TelegramConfig,
    pub log: LogConfig,
}

impl BotConfig {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let s = fs::read_to_string(path)?;
        let cfg: BotConfig = serde_yaml::from_str(&s)?;
        Ok(cfg)
    }
}
