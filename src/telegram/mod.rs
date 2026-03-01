use std::env;
use reqwest::Client;
use tracing::{error, info};
use tokio::sync::mpsc::UnboundedReceiver;

/// Simple Telegram bot that reads configuration from env vars and can send alerts.
pub struct TelegramBot {
    token: String,
    chat_id: i64,
    client: Client,
}

impl TelegramBot {
    /// Create a TelegramBot from the environment. Returns None if required vars are missing.
    pub fn from_env() -> Option<Self> {
        let token = env::var("TELEGRAM_BOT_TOKEN").ok()?;
        let chat_id = env::var("TELEGRAM_CHAT_ID").ok()?;
        let chat_id = chat_id.parse::<i64>().ok()?;
        Some(TelegramBot { token, chat_id, client: Client::new() })
    }

    /// Send a single alert message via the bot.
    pub async fn send_alert(&self, message: &str) -> anyhow::Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        let body = serde_json::json!({"chat_id": self.chat_id, "text": message});
        let resp = self.client.post(&url).json(&body).send().await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let txt = resp.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Telegram send failed: {}", txt))
        }
    }

    /// Spawn a forwarder task that listens on `rx` and forwards messages to Telegram.
    pub fn spawn_forwarder(self, mut rx: UnboundedReceiver<String>) {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = self.send_alert(&msg).await {
                    error!("Telegram send error: {}", e);
                } else {
                    info!("Sent Telegram alert");
                }
            }
            info!("Telegram forwarder exiting");
        });
    }
}
