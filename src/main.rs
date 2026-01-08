//! Prediction Market Arbitrage Bot (MVP)
//!
//! A simplified version of the production arbitrage bot, focused on:
//! - WebSocket price feeds (Kalshi + Polymarket)
//! - Basic arbitrage detection (YES + NO < $1.00)
//! - Dry-run execution (no actual trading)
//! - Position tracking and P&L calculation

use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

mod config;
mod execution;
mod kalshi;
mod polymarket;
mod position_tracker;
mod types;

use config::{
    get_hardcoded_markets, is_dry_run, max_daily_loss_cents, max_position_size,
    WS_RECONNECT_DELAY_SECS,
};
use execution::{check_arbitrage_opportunities, execute_arbitrage_loop};
use position_tracker::PositionTracker;
use types::MarketState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("prediction_market_arbitrage_mvp=info".parse().unwrap()),
        )
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    info!("🚀 Prediction Market Arbitrage Bot (MVP) v0.1.0");
    info!(
        "   Mode: {}",
        if is_dry_run() {
            "DRY RUN (simulation only)"
        } else {
            "LIVE (NOT IMPLEMENTED - will log only)"
        }
    );
    info!("   Max position size: {} contracts", max_position_size());
    info!(
        "   Max daily loss: ${:.2}",
        max_daily_loss_cents() as f64 / 100.0
    );

    // Initialize market state
    let markets = Arc::new(RwLock::new(HashMap::new()));
    for pair in get_hardcoded_markets() {
        let id = pair.id.clone();
        let state = Arc::new(MarketState::new(pair));
        markets.write().unwrap().insert(id, state);
    }

    let market_count = markets.read().unwrap().len();
    info!("   Tracked markets: {}", market_count);

    // Initialize position tracker
    let position_tracker = Arc::new(RwLock::new(PositionTracker::new()));

    // Create arbitrage channel
    let (arb_tx, arb_rx) = mpsc::unbounded_channel();

    // Spawn Kalshi WebSocket task
    let kalshi_markets = markets.clone();
    let kalshi_handle = tokio::spawn(async move {
        loop {
            if let Err(e) = kalshi::run_kalshi_ws(kalshi_markets.clone()).await {
                error!("[KALSHI] WebSocket error: {} - reconnecting...", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(WS_RECONNECT_DELAY_SECS)).await;
        }
    });

    // Spawn Polymarket WebSocket task
    let poly_markets = markets.clone();
    let poly_handle = tokio::spawn(async move {
        loop {
            if let Err(e) = polymarket::run_polymarket_ws(poly_markets.clone()).await {
                error!("[POLYMARKET] WebSocket error: {} - reconnecting...", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(WS_RECONNECT_DELAY_SECS)).await;
        }
    });

    // Spawn arbitrage detection task
    let arb_markets = markets.clone();
    let arb_tracker = position_tracker.clone();
    let arb_detection_handle = tokio::spawn(async move {
        check_arbitrage_opportunities(arb_markets, arb_tracker, arb_tx).await;
    });

    // Spawn execution task
    let exec_tracker = position_tracker.clone();
    let execution_handle = tokio::spawn(async move {
        if let Err(e) = execute_arbitrage_loop(arb_rx, exec_tracker).await {
            error!("[EXECUTION] Error: {}", e);
        }
    });

    // Spawn heartbeat/monitoring task
    let heartbeat_tracker = position_tracker.clone();
    let heartbeat_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let tracker = heartbeat_tracker.read().unwrap();
            info!("💓 System heartbeat | {}", tracker.summary());

            // Check circuit breaker
            let total_loss = -tracker.total_pnl();
            if total_loss > max_daily_loss_cents() as i32 {
                warn!(
                    "⚠️  CIRCUIT BREAKER TRIGGERED | Loss: ${:.2} exceeds limit ${:.2}",
                    total_loss as f64 / 100.0,
                    max_daily_loss_cents() as f64 / 100.0
                );
                warn!("   System would halt in production mode");
            }
        }
    });

    info!("✅ All systems operational");
    info!("   Press Ctrl+C to stop\n");

    // Run until termination
    let _ = tokio::join!(
        kalshi_handle,
        poly_handle,
        arb_detection_handle,
        execution_handle,
        heartbeat_handle
    );

    Ok(())
}
