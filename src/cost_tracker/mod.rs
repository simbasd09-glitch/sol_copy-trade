pub mod balance_monitor;
pub mod ledger;

use crate::cost_tracker::ledger::TradeRecord;
use chrono::Utc;
use std::sync::Mutex;

pub use balance_monitor::BalanceMonitor;

pub struct CostTracker {
    records: Mutex<Vec<TradeRecord>>,
}

impl CostTracker {
    pub fn new() -> Self {
        CostTracker {
            records: Mutex::new(vec![]),
        }
    }

    pub fn add_entry(&self, mint: String, amount: f64, fees: f64) {
        let rec = TradeRecord {
            mint,
            timestamp: Utc::now(),
            amount_spent: amount,
            fees,
            exit_timestamp: None,
            amount_received: None,
        };
        self.records.lock().unwrap().push(rec);
    }

    /// Mark the most recent open trade for `mint` as closed and record amount received.
    pub fn add_exit(&self, mint: String, amount_received: f64, fees: f64) {
        let mut list = self.records.lock().unwrap();
        if let Some(rec) = list.iter_mut().rev().find(|r| r.mint == mint && r.exit_timestamp.is_none()) {
            rec.exit_timestamp = Some(Utc::now());
            rec.amount_received = Some(amount_received);
            rec.fees += fees;
        }
    }

    // more functions omitted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_count() {
        let tracker = CostTracker::new();
        tracker.add_entry("mint1".to_string(), 1.0, 0.0001);
        let count = tracker.records.lock().unwrap().len();
        assert_eq!(count, 1);
    }
}
