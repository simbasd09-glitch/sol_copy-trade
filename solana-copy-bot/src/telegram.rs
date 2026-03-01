use crate::config::BotConfig;
use reqwest::Client;
use std::sync::Arc;

#[derive(Clone)]
pub struct TelegramClient {
    cfg: Arc<BotConfig>,
    client: Client,
}

impl TelegramClient {
    pub fn new(cfg: BotConfig) -> Arc<Self> {
        Arc::new(Self { cfg: Arc::new(cfg), client: Client::new() })
    }

    pub async fn send_message(&self, text: &str) -> anyhow::Result<()> {
        if !self.cfg.telegram.enabled {
            return Ok(());
        }
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.cfg.telegram.bot_token);
        let params = serde_json::json!({"chat_id": self.cfg.telegram.chat_id, "text": text});
        let _ = self.client.post(&url).json(&params).send().await?;
        Ok(())
    }
}
