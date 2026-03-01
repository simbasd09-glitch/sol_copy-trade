use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RpcLatency {
    pub url: String,
    pub last_latency: Duration,
    pub error_rate: f64,
    pub last_checked: Instant,
}

impl RpcLatency {
    pub fn new(url: impl Into<String>) -> Self {
        RpcLatency {
            url: url.into(),
            last_latency: Duration::from_secs(0),
            error_rate: 0.0,
            last_checked: Instant::now(),
        }
    }
}
