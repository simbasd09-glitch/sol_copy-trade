use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeRecord {
    pub mint: String,
    pub timestamp: DateTime<Utc>,
    pub amount_spent: f64,
    pub fees: f64,
    pub exit_timestamp: Option<DateTime<Utc>>,
    pub amount_received: Option<f64>,
}
