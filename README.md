# Prediction Market Arbitrage Bot (MVP)

> 🎯 **MVP Version** - A simplified arbitrage bot for finding risk-free opportunities between Kalshi and Polymarket prediction markets.

This is a minimal viable product (MVP) based on [terauss/Polymarket-Kalshi-Arbitrage-bot](https://github.com/terauss/Polymarket-Kalshi-Arbitrage-bot), focused on core arbitrage detection without the production optimizations.

## How It Works

In prediction markets, **YES + NO = $1.00** (guaranteed). When you can buy both sides for less than $1.00:

```
Example:
  Kalshi YES:  $0.42
  Poly NO:     $0.56
  Total cost:  $0.98
  Payout:      $1.00
  Profit:      $0.02 (2% risk-free return)
```

## MVP Scope

### ✅ What's Included

- **WebSocket price feeds** from Kalshi and Polymarket
- **Basic arbitrage detection** (YES + NO < $1.00)
- **Kalshi fee calculation** (~2¢ per contract)
- **Dry-run mode** (logs opportunities, no actual trading)
- **Position tracking** (P&L calculation)
- **Simple circuit breaker** (max position limits)
- **Hardcoded market list** (no dynamic discovery)

### ❌ What's NOT Included (vs. Parent)

- SIMD-accelerated detection (uses simple if-statements)
- Lock-free atomic orderbooks (uses Arc<RwLock>)
- Dynamic market discovery (hardcoded markets)
- Advanced risk management (basic caps only)
- Actual order execution (dry-run only)
- Polymarket CLOB client (no trading)
- RSA signature generation (no private APIs)

## Quick Start

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Clone & Configure

```bash
git clone https://github.com/miqq27/prediction-market-arbitrage-mvp.git
cd prediction-market-arbitrage-mvp

# Copy environment template
cp .env.example .env
```

### 3. Run (Dry Mode)

```bash
RUST_LOG=info cargo run --release
```

The bot will:
1. Connect to Kalshi and Polymarket WebSocket feeds
2. Monitor hardcoded markets for arbitrage opportunities
3. Log detected opportunities (no actual trading)
4. Track hypothetical P&L

## Configuration

Edit `.env`:

```bash
# Dry run mode (1 = simulation only, 0 = live trading - NOT IMPLEMENTED IN MVP)
DRY_RUN=1

# Logging level
RUST_LOG=info

# Risk limits
MAX_POSITION_SIZE=10
MAX_DAILY_LOSS=5000
```

## Understanding the Output

```
🚀 Prediction Market Arbitrage MVP v0.1.0
   Mode: DRY RUN
   Tracked markets: 3

[INFO] Connected to Kalshi WebSocket
[INFO] Connected to Polymarket WebSocket
[INFO] Market update: KXEPLGAME-25DEC27CFCARS | Kalshi YES=42¢ NO=58¢
[INFO] Market update: chelsea-vs-arsenal | Poly YES=40¢ NO=56¢

🎯 ARBITRAGE DETECTED!
   Market: Chelsea vs Arsenal
   Strategy: Kalshi YES (42¢) + Poly NO (56¢)
   Total cost: 98¢ (incl. 2¢ Kalshi fee)
   Profit: 2¢ (2.04% return)
   [DRY RUN - Not executing]
```

## Architecture

```
src/
├── main.rs              # Entry point, spawn WebSocket listeners
├── types.rs             # Market state, ArbType, price structures
├── config.rs            # Hardcoded market list + thresholds
├── kalshi.rs            # Kalshi WebSocket client
├── polymarket.rs        # Polymarket WebSocket client  
├── execution.rs         # Arbitrage detection + dry-run logging
├── position_tracker.rs  # P&L tracking
└── lib.rs               # Module declarations
```

## Hardcoded Markets (MVP)

The MVP tracks these sample markets (edit `src/config.rs` to change):

1. **Chelsea vs Arsenal** (EPL)
   - Kalshi: `KXEPLGAME-25DEC27CFCARS-CFC`
   - Polymarket: `chelsea-vs-arsenal`

2. **Lakers vs Celtics** (NBA)
   - Kalshi: `KXNBAGAME-25JAN15LALCEL-LAL`
   - Polymarket: `lakers-vs-celtics`

3. **Bitcoin > $100k** (Crypto)
   - Kalshi: `KXBTC-25FEB01-100K`
   - Polymarket: `bitcoin-100k-feb-2025`

## Next Steps (Beyond MVP)

To evolve this into a production bot:

1. **Add actual execution**
   - Implement Kalshi REST API with RSA signing
   - Add Polymarket CLOB order submission
   - Handle partial fills and rejections

2. **Optimize performance**
   - Replace `RwLock` with atomic operations
   - Add SIMD for batch price checks
   - Reduce WebSocket message parsing overhead

3. **Add market discovery**
   - Query Kalshi events API
   - Match with Polymarket markets via team mappings
   - Auto-subscribe to new opportunities

4. **Improve risk management**
   - Per-market position limits
   - Consecutive error tracking
   - Auto-halt on anomalies

5. **Add monitoring**
   - Prometheus metrics
   - Grafana dashboards
   - Slack/Discord alerts

## Disclaimer

⚠️ **Educational Purpose Only**

This MVP is for learning arbitrage concepts and WebSocket handling. It does NOT execute real trades. Before running any trading bot with real money:

- Understand exchange APIs, fees, and rate limits
- Test thoroughly in paper trading mode
- Implement proper error handling and monitoring
- Comply with exchange terms of service
- Never risk more than you can afford to lose

## License

MIT License - see parent repository for details.

## Credits

Based on [terauss/Polymarket-Kalshi-Arbitrage-bot](https://github.com/terauss/Polymarket-Kalshi-Arbitrage-bot) - a production-grade arbitrage system with sub-millisecond latency.
