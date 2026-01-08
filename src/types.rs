//! Core type definitions for the arbitrage trading system.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// Price in cents (1-99 for $0.01-$0.99), 0 means no price available
pub type PriceCents = u16;

/// Size in cents (dollar amount × 100)
pub type SizeCents = u16;

/// Sentinel value for no price
pub const NO_PRICE: PriceCents = 0;

/// Market category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketType {
    Moneyline,
    Spread,
    Total,
}

impl std::fmt::Display for MarketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketType::Moneyline => write!(f, "moneyline"),
            MarketType::Spread => write!(f, "spread"),
            MarketType::Total => write!(f, "total"),
        }
    }
}

/// A matched trading pair between Kalshi and Polymarket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketPair {
    pub id: String,
    pub description: String,
    pub market_type: MarketType,
    pub kalshi_ticker: String,
    pub poly_slug: String,
    pub poly_yes_token: String,
    pub poly_no_token: String,
}

/// Orderbook state for a single platform
#[derive(Debug, Clone, Default)]
pub struct Orderbook {
    pub yes_ask: PriceCents,
    pub no_ask: PriceCents,
    pub yes_size: SizeCents,
    pub no_size: SizeCents,
}

/// Market state tracking both platforms
#[derive(Debug, Clone)]
pub struct MarketState {
    pub pair: MarketPair,
    pub kalshi: Arc<RwLock<Orderbook>>,
    pub poly: Arc<RwLock<Orderbook>>,
}

impl MarketState {
    pub fn new(pair: MarketPair) -> Self {
        Self {
            pair,
            kalshi: Arc::new(RwLock::new(Orderbook::default())),
            poly: Arc::new(RwLock::new(Orderbook::default())),
        }
    }
}

/// Arbitrage opportunity type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArbType {
    /// Buy Polymarket YES + Buy Kalshi NO
    PolyYesKalshiNo,
    /// Buy Kalshi YES + Buy Polymarket NO
    KalshiYesPolyNo,
    /// Buy Polymarket YES + Buy Polymarket NO (rare)
    PolyOnly,
    /// Buy Kalshi YES + Buy Kalshi NO (rare)
    KalshiOnly,
}

impl std::fmt::Display for ArbType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArbType::PolyYesKalshiNo => write!(f, "Poly YES + Kalshi NO"),
            ArbType::KalshiYesPolyNo => write!(f, "Kalshi YES + Poly NO"),
            ArbType::PolyOnly => write!(f, "Poly YES + Poly NO"),
            ArbType::KalshiOnly => write!(f, "Kalshi YES + Kalshi NO"),
        }
    }
}

/// Arbitrage opportunity
#[derive(Debug, Clone)]
pub struct ArbOpportunity {
    pub market_id: String,
    pub description: String,
    pub arb_type: ArbType,
    pub yes_price: PriceCents,
    pub no_price: PriceCents,
    pub total_cost: PriceCents,
    pub fee: PriceCents,
    pub profit: i16,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Calculate Kalshi trading fee in cents
/// Formula: ceil(0.07 × P × (1-P)) in cents
#[inline]
pub fn kalshi_fee_cents(price_cents: PriceCents) -> PriceCents {
    if price_cents == 0 || price_cents >= 100 {
        return 0;
    }
    let p = price_cents as f64 / 100.0;
    ((0.07 * p * (1.0 - p) * 100.0).ceil() as u16).max(1)
}

/// Convert f64 price (0.01-0.99) to PriceCents (1-99)
#[inline]
pub fn price_to_cents(price: f64) -> PriceCents {
    ((price * 100.0).round() as u16).clamp(0, 99)
}

/// Convert PriceCents to f64
#[inline]
pub fn cents_to_price(cents: PriceCents) -> f64 {
    cents as f64 / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalshi_fee_cents() {
        // At 50 cents: ceil(7 * 50 * 50 / 10000) = ceil(1.75) = 2
        assert_eq!(kalshi_fee_cents(50), 2);
        // At 10 cents: ceil(7 * 10 * 90 / 10000) = ceil(0.63) = 1
        assert_eq!(kalshi_fee_cents(10), 1);
        // At 0 and 100: no fee
        assert_eq!(kalshi_fee_cents(0), 0);
        assert_eq!(kalshi_fee_cents(100), 0);
    }

    #[test]
    fn test_price_conversion() {
        assert_eq!(price_to_cents(0.50), 50);
        assert_eq!(price_to_cents(0.01), 1);
        assert_eq!(price_to_cents(0.99), 99);
        assert!((cents_to_price(50) - 0.50).abs() < 0.001);
    }
}
