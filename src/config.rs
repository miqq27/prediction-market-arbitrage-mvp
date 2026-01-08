//! Configuration and hardcoded market definitions for MVP.

use crate::types::{MarketPair, MarketType};

/// Arbitrage threshold in cents (100 = $1.00)
pub const ARB_THRESHOLD_CENTS: u16 = 100;

/// WebSocket reconnect delay in seconds
pub const WS_RECONNECT_DELAY_SECS: u64 = 5;

/// Hardcoded market list for MVP
/// In production, this would be dynamically discovered
pub fn get_hardcoded_markets() -> Vec<MarketPair> {
    vec![
        MarketPair {
            id: "chelsea-arsenal".into(),
            description: "Chelsea vs Arsenal (EPL)".into(),
            market_type: MarketType::Moneyline,
            kalshi_ticker: "KXEPLGAME-25DEC27CFCARS-CFC".into(),
            poly_slug: "chelsea-vs-arsenal".into(),
            poly_yes_token: "0x123...abc".into(), // Placeholder
            poly_no_token: "0x456...def".into(),  // Placeholder
        },
        MarketPair {
            id: "lakers-celtics".into(),
            description: "Lakers vs Celtics (NBA)".into(),
            market_type: MarketType::Moneyline,
            kalshi_ticker: "KXNBAGAME-25JAN15LALCEL-LAL".into(),
            poly_slug: "lakers-vs-celtics".into(),
            poly_yes_token: "0x789...ghi".into(), // Placeholder
            poly_no_token: "0xabc...jkl".into(),  // Placeholder
        },
        MarketPair {
            id: "bitcoin-100k".into(),
            description: "Bitcoin > $100k (Feb 2025)".into(),
            market_type: MarketType::Total,
            kalshi_ticker: "KXBTC-25FEB01-100K".into(),
            poly_slug: "bitcoin-100k-feb-2025".into(),
            poly_yes_token: "0xdef...mno".into(), // Placeholder
            poly_no_token: "0xghi...pqr".into(),  // Placeholder
        },
    ]
}

/// Get max position size from environment (default: 10 contracts)
pub fn max_position_size() -> u16 {
    std::env::var("MAX_POSITION_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10)
}

/// Get max daily loss in cents from environment (default: $50.00)
pub fn max_daily_loss_cents() -> u16 {
    std::env::var("MAX_DAILY_LOSS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5000)
}

/// Check if dry run mode is enabled (default: true)
pub fn is_dry_run() -> bool {
    std::env::var("DRY_RUN")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(true)
}
