use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub dev_wallets: Vec<String>,
    pub buy_amount_sol: f64,
    pub slippage_bps: u16,
    pub priority_fee_multiplier: f64,
    pub min_market_cap: u64,
    pub max_market_cap: u64,
    pub stop_loss_percent: f64,
    pub take_profit_percent: f64,
    pub sell_delay_seconds: u64,
    pub jito_tip_lamports: u64,
    pub min_balance_threshold_sol: f64,
    pub max_concurrent_trades: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            dev_wallets: vec![],
            buy_amount_sol: 0.01,
            slippage_bps: 1000,
            priority_fee_multiplier: 1.5,
            min_market_cap: 2000,
            max_market_cap: 4000,
            stop_loss_percent: 10.0,
            take_profit_percent: 50.0,
            sell_delay_seconds: 0,
            jito_tip_lamports: 100000,
            min_balance_threshold_sol: 0.05,
            max_concurrent_trades: 5,
        }
    }
}

impl Settings {
    pub async fn load(path: &str) -> Result<Self, anyhow::Error> {
        let content = fs::read_to_string(path)?;
        let settings: Settings = toml::from_str(&content)?;
        Ok(settings)
    }

    pub async fn save(&self, path: &str) -> Result<(), anyhow::Error> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_default_and_persistence() {
        let s = Settings::default();
        let path = "test_settings.toml";
        s.save(path).await.unwrap();
        let loaded = Settings::load(path).await.unwrap();
        assert_eq!(s.buy_amount_sol, loaded.buy_amount_sol);
        fs::remove_file(path).unwrap();
    }
}
