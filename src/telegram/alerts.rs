use chrono::{DateTime, Utc};
use teloxide::prelude::*;

pub async fn send_message(bot: &Bot, chat_id: ChatId, text: &str) {
    let _ = bot.send_message(chat_id, text.to_string()).await;
}

pub async fn format_buy(
    symbol: &str,
    amount: f64,
    price: f64,
    market_cap: u64,
    tx_link: &str,
) -> String {
    format!("🟢 Bought {} for {} SOL (price {})\nmarket cap {}\n{}", symbol, amount, price, market_cap, tx_link)
}

pub async fn format_sell(
    symbol: &str,
    profit: f64,
    fees: f64,
    tx_link: &str,
) -> String {
    format!("🔴 Sold {} profit {}% fees {}\n{}", symbol, profit, fees, tx_link)
}

pub async fn format_error(err: &str) -> String {
    format!("⚠️ Error: {}", err)
}
