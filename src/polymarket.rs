//! Polymarket WebSocket client for orderbook updates.
//!
//! MVP version: connects to public WebSocket feed, no authentication.
//! Does NOT support order execution (would require CLOB client integration).

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use crate::types::{price_to_cents, MarketState, Orderbook, PriceCents, SizeCents};

/// Polymarket WebSocket URL (public orderbook feed)
const POLYMARKET_WS_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";

/// Run Polymarket WebSocket connection
pub async fn run_polymarket_ws(
    markets: Arc<RwLock<HashMap<String, Arc<MarketState>>>>,
) -> Result<()> {
    info!("[POLYMARKET] Connecting to WebSocket: {}", POLYMARKET_WS_URL);

    let (ws_stream, _) = connect_async(POLYMARKET_WS_URL)
        .await
        .context("Failed to connect to Polymarket WebSocket")?;

    info!("[POLYMARKET] ✅ Connected to WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to orderbook updates for all tracked markets
    let markets_guard = markets.read().unwrap();
    let token_ids: Vec<String> = markets_guard
        .values()
        .flat_map(|m| vec![m.pair.poly_yes_token.clone(), m.pair.poly_no_token.clone()])
        .collect();
    drop(markets_guard);

    if !token_ids.is_empty() {
        for token_id in &token_ids {
            let subscribe_msg = serde_json::json!({
                "type": "subscribe",
                "channel": "book",
                "market": token_id,
            });

            write
                .send(Message::Text(subscribe_msg.to_string()))
                .await
                .context("Failed to send subscribe message")?;
        }

        info!("[POLYMARKET] Subscribed to {} tokens", token_ids.len());
    }

    // Read messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) = handle_polymarket_message(&text, &markets) {
                    warn!("[POLYMARKET] Error handling message: {}", e);
                }
            }
            Ok(Message::Ping(data)) => {
                debug!("[POLYMARKET] Received ping, sending pong");
                if let Err(e) = write.send(Message::Pong(data)).await {
                    error!("[POLYMARKET] Failed to send pong: {}", e);
                }
            }
            Ok(Message::Close(_)) => {
                warn!("[POLYMARKET] WebSocket closed by server");
                break;
            }
            Err(e) => {
                error!("[POLYMARKET] WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    warn!("[POLYMARKET] WebSocket disconnected");
    Ok(())
}

/// Handle incoming Polymarket message
fn handle_polymarket_message(
    text: &str,
    markets: &Arc<RwLock<HashMap<String, Arc<MarketState>>>>,
) -> Result<()> {
    let msg: Value = serde_json::from_str(text).context("Failed to parse JSON")?;

    // Check event type
    let event_type = msg
        .get("event_type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    match event_type {
        "book" => {
            handle_book_update(&msg, markets)?;
        }
        "subscribed" => {
            debug!("[POLYMARKET] Subscription confirmed");
        }
        "error" => {
            warn!("[POLYMARKET] Error message: {:?}", msg);
        }
        _ => {
            debug!("[POLYMARKET] Unknown event type: {}", event_type);
        }
    }

    Ok(())
}

/// Handle book update
fn handle_book_update(
    msg: &Value,
    markets: &Arc<RwLock<HashMap<String, Arc<MarketState>>>>,
) -> Result<()> {
    let token_id = msg
        .get("market")
        .and_then(|v| v.as_str())
        .context("Missing market/token_id")?;

    let markets_guard = markets.read().unwrap();
    let market_state = markets_guard.values().find(|m| {
        m.pair.poly_yes_token == token_id || m.pair.poly_no_token == token_id
    });

    let market_state = match market_state {
        Some(m) => m.clone(),
        None => return Ok(()), // Market not tracked
    };
    drop(markets_guard);

    let is_yes = market_state.pair.poly_yes_token == token_id;

    // Parse best ask price (Polymarket uses "0.XX" format)
    let asks = msg.get("asks").and_then(|v| v.as_array());
    let best_ask_price: PriceCents = if let Some(asks) = asks {
        asks.first()
            .and_then(|order| order.get("price"))
            .and_then(|p| p.as_str())
            .map(|s| {
                s.parse::<f64>()
                    .ok()
                    .map(price_to_cents)
                    .unwrap_or(0)
            })
            .unwrap_or(0)
    } else {
        0
    };

    // Parse best ask size (in dollars, convert to cents)
    let best_ask_size: SizeCents = if let Some(asks) = asks {
        asks.first()
            .and_then(|order| order.get("size"))
            .and_then(|s| s.as_str())
            .map(|s| {
                s.parse::<f64>()
                    .ok()
                    .map(|sz| (sz * 100.0) as u16)
                    .unwrap_or(0)
            })
            .unwrap_or(0)
    } else {
        0
    };

    // Update market state
    {
        let mut book = market_state.poly.write().unwrap();
        if is_yes {
            book.yes_ask = best_ask_price;
            book.yes_size = best_ask_size;
        } else {
            book.no_ask = best_ask_price;
            book.no_size = best_ask_size;
        }
    }

    debug!(
        "[POLYMARKET] {} | {} ask: {}¢ ({}¢)",
        market_state.pair.description,
        if is_yes { "YES" } else { "NO" },
        best_ask_price,
        best_ask_size
    );

    Ok(())
}
