//! Arbitrage detection and execution logic.
//!
//! MVP version: detects arbitrage opportunities and logs them (dry-run only).
//! Does NOT execute actual trades.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::config::{is_dry_run, max_position_size, ARB_THRESHOLD_CENTS};
use crate::position_tracker::PositionTracker;
use crate::types::{
    kalshi_fee_cents, ArbOpportunity, ArbType, MarketState, NO_PRICE,
};

/// Check all markets for arbitrage opportunities
pub async fn check_arbitrage_opportunities(
    markets: Arc<RwLock<HashMap<String, Arc<MarketState>>>>,
    position_tracker: Arc<RwLock<PositionTracker>>,
    arb_tx: mpsc::UnboundedSender<ArbOpportunity>,
) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));

    loop {
        interval.tick().await;

        let markets_guard = markets.read().unwrap();
        for market in markets_guard.values() {
            if let Some(arb) = detect_arbitrage(market) {
                // Check position limits
                let tracker = position_tracker.read().unwrap();
                if tracker.can_trade(&market.pair.id, max_position_size()) {
                    drop(tracker);
                    let _ = arb_tx.send(arb);
                } else {
                    warn!(
                        "[ARB] Position limit reached for {}",
                        market.pair.description
                    );
                }
            }
        }
        drop(markets_guard);
    }
}

/// Detect arbitrage opportunity for a single market
fn detect_arbitrage(market: &MarketState) -> Option<ArbOpportunity> {
    let kalshi = market.kalshi.read().unwrap();
    let poly = market.poly.read().unwrap();

    let k_yes = kalshi.yes_ask;
    let k_no = kalshi.no_ask;
    let p_yes = poly.yes_ask;
    let p_no = poly.no_ask;

    // Skip if any price is missing
    if k_yes == NO_PRICE || k_no == NO_PRICE || p_yes == NO_PRICE || p_no == NO_PRICE {
        return None;
    }

    // Check all 4 possible arbitrage combinations
    let opportunities = vec![
        // Cross-platform: Poly YES + Kalshi NO
        (
            ArbType::PolyYesKalshiNo,
            p_yes,
            k_no,
            kalshi_fee_cents(k_no),
        ),
        // Cross-platform: Kalshi YES + Poly NO
        (
            ArbType::KalshiYesPolyNo,
            k_yes,
            p_no,
            kalshi_fee_cents(k_yes),
        ),
        // Same-platform: Poly YES + Poly NO (no fees)
        (ArbType::PolyOnly, p_yes, p_no, 0),
        // Same-platform: Kalshi YES + Kalshi NO (double fees)
        (
            ArbType::KalshiOnly,
            k_yes,
            k_no,
            kalshi_fee_cents(k_yes) + kalshi_fee_cents(k_no),
        ),
    ];

    // Find best arbitrage opportunity
    let mut best: Option<ArbOpportunity> = None;

    for (arb_type, yes_price, no_price, fee) in opportunities {
        let total_cost = yes_price + no_price + fee;

        if total_cost < ARB_THRESHOLD_CENTS {
            let profit = ARB_THRESHOLD_CENTS as i16 - total_cost as i16;

            let arb = ArbOpportunity {
                market_id: market.pair.id.clone(),
                description: market.pair.description.clone(),
                arb_type,
                yes_price,
                no_price,
                total_cost,
                fee,
                profit,
                timestamp: chrono::Utc::now(),
            };

            if best.is_none() || profit > best.as_ref().unwrap().profit {
                best = Some(arb);
            }
        }
    }

    best
}

/// Execute arbitrage opportunities (dry-run only in MVP)
pub async fn execute_arbitrage_loop(
    mut arb_rx: mpsc::UnboundedReceiver<ArbOpportunity>,
    position_tracker: Arc<RwLock<PositionTracker>>,
) -> Result<()> {
    let dry_run = is_dry_run();

    while let Some(arb) = arb_rx.recv().await {
        info!(
            "\n🎯 ARBITRAGE DETECTED!\n   Market: {}\n   Strategy: {}\n   YES: {}¢ | NO: {}¢ | Fee: {}¢\n   Total cost: {}¢\n   Profit: {}¢ ({:.2}%)\n   {}",
            arb.description,
            arb.arb_type,
            arb.yes_price,
            arb.no_price,
            arb.fee,
            arb.total_cost,
            arb.profit,
            (arb.profit as f64 / arb.total_cost as f64) * 100.0,
            if dry_run { "[DRY RUN - Not executing]" } else { "[EXECUTING]" }
        );

        if dry_run {
            // In dry-run mode, just log and track hypothetical position
            let mut tracker = position_tracker.write().unwrap();
            tracker.record_trade(&arb.market_id, arb.profit);
        } else {
            // In live mode, this would execute actual trades
            warn!("[EXECUTION] Live trading NOT implemented in MVP");
        }
    }

    Ok(())
}
