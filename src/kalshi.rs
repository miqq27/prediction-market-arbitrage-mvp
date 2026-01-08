//! Kalshi WebSocket client for orderbook updates.
//!
//! MVP version: connects to public WebSocket feed, no authentication.
//! Does NOT support order execution (would require RSA signature generation).

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use crate::types::{MarketState, Orderbook, PriceCents, SizeCents};

/// Kalshi WebSocket URL (demo/public endpoint)
const KALSHI_WS_URL: &str = "wss://demo-api.kalshi.co/trade-api/ws/v2";

/// Run Kalshi WebSocket connection
pub async fn run_kalshi_ws(
    markets: Arc<RwLock<HashMap<String, Arc<MarketState>>>>,
) -> Result<()> {
    info!("[KALSHI] Connecting to WebSocket: {}", KALSHI_WS_URL);

    let (ws_stream, _) = connect_async(KALSHI_WS_URL)
        .await
        .context("Failed to connect to Kalshi WebSocket")?;

    info!("[KALSHI] ✅ Connected to WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to orderbook updates for all tracked markets
    let markets_guard = markets.read().unwrap();
    let tickers: Vec<String> = markets_guard
        .values()
        .map(|m| m.pair.kalshi_ticker.clone())
        .collect();
    drop(markets_guard);

    if !tickers.is_empty() {
        let subscribe_msg = serde_json::json!({
            "type": "subscribe",
            "channels": [{
                "name": "orderbook_delta",
                "tickers": tickers,
            }]
        });

        write
            .send(Message::Text(subscribe_msg.to_string()))
            .await
            .context("Failed to send subscribe message")?;

        info!("[KALSHI] Subscribed to {} markets", tickers.len());
    }

    // Read messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) = handle_kalshi_message(&text, &markets) {
                    warn!("[KALSHI] Error handling message: {}", e);
                }
            }
            Ok(Message::Ping(data)) => {
                debug!("[KALSHI] Received ping, sending pong");
                if let Err(e) = write.send(Message::Pong(data)).await {
                    error!("[KALSHI] Failed to send pong: {}", e);
                }
            }
            Ok(Message::Close(_)) => {
                warn!("[KALSHI] WebSocket closed by server");
                break;
            }
            Err(e) => {
                error!("[KALSHI] WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    warn!("[KALSHI] WebSocket disconnected");
    Ok(())
}

/// Handle incoming Kalshi message
fn handle_kalshi_message(
    text: &str,
    markets: &Arc<RwLock<HashMap<String, Arc<MarketState>>>>,
) -> Result<()> {
    let msg: Value = serde_json::from_str(text).context("Failed to parse JSON")?;

    // Check message type
    let msg_type = msg
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    match msg_type {
        "orderbook_delta" => {
            handle_orderbook_delta(&msg, markets)?;
        }
        "subscribed" => {
            debug!("[KALSHI] Subscription confirmed");
        }
        "error" => {
            warn!("[KALSHI] Error message: {:?}", msg);
        }
        _ => {
            debug!("[KALSHI] Unknown message type: {}", msg_type);
        }
    }

    Ok(())
}

/// Handle orderbook delta update
fn handle_orderbook_delta(
    msg: &Value,
    markets: &Arc<RwLock<HashMap<String, Arc<MarketState>>>>,
) -> Result<()> {
    let ticker = msg
        .get("ticker")
        .and_then(|v| v.as_str())
        .context("Missing ticker")?;

    let markets_guard = markets.read().unwrap();
    let market_state = markets_guard
        .values()
        .find(|m| m.pair.kalshi_ticker == ticker);

    let market_state = match market_state {
        Some(m) => m.clone(),
        None => return Ok(()), // Market not tracked
    };
    drop(markets_guard);

    // Parse yes_ask and no_ask (prices are in cents: 1-99)
    let yes_ask = msg
        .get("yes_ask")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as PriceCents;

    let no_ask = msg
        .get("no_ask")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as PriceCents;

    // Parse sizes (in cents)
    let yes_size = msg
        .get("yes_ask_size")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as SizeCents;

    let no_size = msg
        .get("no_ask_size")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as SizeCents;

    // Update market state
    {
        let mut book = market_state.kalshi.write().unwrap();
        book.yes_ask = yes_ask;
        book.no_ask = no_ask;
        book.yes_size = yes_size;
        book.no_size = no_size;
    }

    debug!(
        "[KALSHI] {} | YES: {}¢ ({}¢) | NO: {}¢ ({}¢)",
        market_state.pair.description, yes_ask, yes_size, no_ask, no_size
    );

    Ok(())
}
