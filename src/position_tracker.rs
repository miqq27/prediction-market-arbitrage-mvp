//! Position tracking and P&L calculation.

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct PositionTracker {
    /// Market ID -> position size (in contracts)
    positions: HashMap<String, u16>,
    /// Total P&L in cents
    total_pnl: i32,
    /// Trade count
    trade_count: u32,
}

impl PositionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if we can trade (within position limits)
    pub fn can_trade(&self, market_id: &str, max_size: u16) -> bool {
        let current = self.positions.get(market_id).copied().unwrap_or(0);
        current < max_size
    }

    /// Record a trade (dry-run or actual)
    pub fn record_trade(&mut self, market_id: &str, profit_cents: i16) {
        *self.positions.entry(market_id.to_string()).or_insert(0) += 1;
        self.total_pnl += profit_cents as i32;
        self.trade_count += 1;
    }

    /// Get current position for a market
    pub fn get_position(&self, market_id: &str) -> u16 {
        self.positions.get(market_id).copied().unwrap_or(0)
    }

    /// Get total P&L in cents
    pub fn total_pnl(&self) -> i32 {
        self.total_pnl
    }

    /// Get trade count
    pub fn trade_count(&self) -> u32 {
        self.trade_count
    }

    /// Get P&L summary
    pub fn summary(&self) -> String {
        format!(
            "Trades: {} | P&L: ${:.2} | Positions: {}",
            self.trade_count,
            self.total_pnl as f64 / 100.0,
            self.positions.len()
        )
    }
}
