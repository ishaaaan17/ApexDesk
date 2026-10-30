# ApexDesk - System Architecture & Engineering Blueprint 🏛️

> **Scale Target:** 10,000+ Concurrent Users  
> **Tech Stack:** 100% Rust (Sub-microsecond, Memory-Safe, Zero Garbage Collection)  
> **Market Target:** Indian Equity Market (NSE / BSE)  
> **Repository:** [github.com/ishaaaan17/ApexDesk](https://github.com/ishaaaan17/ApexDesk)  
> **Maintainer:** Ishaan Maheshwari ([@ishaaaan17](https://github.com/ishaaaan17))

---

## 1. High-Level System Architecture

```
                                  [ FREE INDIAN MARKET DATA ]
                           (NSE Live Feeds, Ticks, 5m/15m Candles)
                                              │
                                              ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        LAYER 1: MARKET INGESTION & QUANT AGGREGATOR                    │
│   • Tick to Multi-Timeframe OHLCV Aggregator (1m, 5m, 15m)                             │
│   • 15-Minute Opening Range (ORB) Calculator (High, Low, Mid)                          │
│   • Quant Indicators: 20 DMA, 50 DMA, 200 DMA, 14-period ATR, VWAP                    │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                     LAYER 2: MULTI-FACTOR QUANTITATIVE SCREENER                        │
│   • Multi-Factor Scoring (0 to 100 Points) across Watchlist                            │
│   • Factors: Primary Trend (35%), Breakout Proximity (25%), Volume Surge vs ADV (20%), │
│              Volatility ATR % (10%), AI Learned Historical Weight (10%)                 │
│   • Automatic Capital Allocation (%) Generator                                         │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                   LAYER 3: PRE-TRADE RISK ENGINE & CIRCUIT GUARDS                      │
│   • Max 1% Risk-per-Trade Fractional Sizing Formula                                    │
│   • Indian Market Upper/Lower Circuit Guards (Blocks trades near 5%, 10%, 20% bands)   │
│   • Daily Max Drawdown Hard Stop (2% Daily Loss = Auto System Shutdown)                │
│   • Trailing Stop-Loss Engine (+1.0R -> Breakeven Lock, +1.5R -> Profit Lock)          │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│               LAYER 4: HIGH-THROUGHPUT EXECUTION & MATCHING ENGINE                     │
│   • Bracket Order State Machine (Market, Limit, Stop-Loss, Take-Profit)                │
│   • L2 Price-Time Priority In-Memory Order Book                                        │
│   • Realistic Indian Slippage Simulator (0.03% - 0.05%)                                │
│   • Regulatory Tax Engine: STT, GST, SEBI fee, Stamp Duty, Brokerage (Paise precision) │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│            LAYER 5: MULTI-TENANT LEDGER & PORTFOLIO (10,000+ Users)                    │
│   • Multi-Tenant User Isolation (`user_id`, `account_id`)                              │
│   • Thread-Safe Double-Entry Bookkeeping (Every Debit == Credit)                       │
│   • FIFO Position Cost Tracking & Real-Time Mark-to-Market (MTM) P&L                   │
│   • Fixed-Point `Money` Type (Zero floating point rounding drift)                      │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│               LAYER 6: CONTINUOUS REINFORCEMENT LEARNING BRAIN                         │
│   • Immutable Trade Journal (Records every fill, fee, R-multiple, duration)            │
│   • End-of-Day (EOD) Walk-Forward Policy Update                                        │
│   • Win-Rate Bandit: Boosts weights of clean breakout stocks, downweights false wicks  │
│   • Drawdown Defense: Dynamically scales down account risk multiplier in rough regimes │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│               LAYER 7: NOTIFICATIONS, API & DISASTER RECOVERY                          │
│   • 100% Free Telegram Bot Alerts (Instant order tickets, stop triggers, EOD P&L)      │
│   • Emergency Panic Kill-Switch (`--kill-all`: Cancels and closes all open positions)  │
│   • REST & WebSocket API (High-concurrency async Tokio / Axum backend)                 │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Complete 4-Week Detailed Implementation Roadmap

### 📅 Week 1: The Institutional Quant Core (18 Micro-Features)
* [x] **1.1 Indian Regulatory Tax Engine** (`src/taxes.rs`): STT, GST, SEBI, Stamp Duty, Exchange charges, Brokerage.
* [x] **1.2 Fixed-Point Money Type** (`src/types/money.rs`): Paise-precision integer math, zero float-rounding bugs.
* [ ] **1.3 Precision Rounding & Math Helpers** (`src/types/math.rs`): Fixed-point division, BPS multipliers, and basis ratios.
* [ ] **1.4 Multi-Tenant User & Account State Model** (`src/models/account.rs`): Thread-safe isolated account balances.
* [ ] **1.5 Double-Entry Accounting Ledger** (`src/models/ledger.rs`): Immutable financial audit trail (`debit == credit`).
* [ ] **1.6 Portfolio & Position Tracker** (`src/models/portfolio.rs`): Real-time Mark-to-Market (MTM) & FIFO cost accounting.
* [ ] **1.7 Bracket, Market, Limit & Stop Order State Machine** (`src/models/order.rs`): Order lifecycle states.
* [ ] **1.8 In-Memory Order Book** (`src/orderbook/mod.rs`): Price-Time priority matching queue.
* [ ] **1.9 High-Throughput Matching Engine** (`src/broker/matching_engine.rs`): Simulates real fills, partial fills & slippage.
* [ ] **1.10 Pre-Trade Risk Engine** (`src/risk/pre_trade.rs`): 1% max risk cap per trade and margin verification.
* [ ] **1.11 Daily Loss Circuit Breaker & Kill-Switch** (`src/risk/circuit_breaker.rs`): Emergency panic button.
* [ ] **1.12 Indian Market Upper/Lower Circuit Guard** (`src/risk/market_circuits.rs`): 5%, 10%, 20% circuit protection.
* [ ] **1.13 Trailing Stop-Loss Engine** (`src/risk/trailing_stop.rs`): Breakeven auto-lock (+1R) and trailing profit lock (+1.5R).
* [ ] **1.14 Multi-Timeframe Candle Aggregator** (`src/indicators/bars.rs`): Aggregates ticks to 1m, 5m, 15m OHLCV bars.
* [ ] **1.15 Opening Range Breakout (ORB) Engine** (`src/indicators/orb.rs`): 15m/30m High/Low/Mid breakout calculation.
* [ ] **1.16 Quant Math Engine** (`src/indicators/math.rs`): 20/50 DMA, 14-period ATR, and VWAP.
* [ ] **1.17 Multi-Threaded Stress Test** (`tests/concurrency_test.rs`): Simulates 1,000+ simultaneous virtual orders.
* [ ] **1.18 Sub-Microsecond Performance Benchmarks** (`benches/core_bench.rs`): Nanosecond benchmarking with `cargo bench`.

---

### 📅 Week 2: Advanced Pre-Trade Risk & Market Intelligence
* **2.1 NIFTY 50 Market Regime Filter**: Blocks long breakouts when the broader market is dumping (-1% below VWAP).
* **2.2 Quarterly Earnings Event Blackout Filter**: Automatically bans stocks on their quarterly results date to avoid high-gap risk.
* **2.3 Multi-Timeframe Re-Test Confirmation Engine**: Requires a 5-minute pullback retest holding above 15m ORB high before firing.
* **2.4 Volatility Squeeze Detector**: Bollinger Bands inside Keltner Channels to identify explosive upcoming moves.
* **2.5 Automated EOD Square-Off Daemon**: Strict 3:15 PM IST auto-close engine preventing intraday MIS broker penalties.

---

### 📅 Week 3: Market Data Feeds, Streaming & Notifications
* **3.1 100% Free Live NSE Tick Streamer**: High-speed, polite pollers and WebSocket connections for real-time quotes.
* **3.2 Custom Watchlist Manager**: CLI, file-based (`custom_watchlist.txt`), and programmatic dynamic watchlists.
* **3.3 Free Telegram Notification Bot**: Sends instant real-time order execution tickets, stop-loss triggers, and daily P&L.
* **3.4 Windows Background Scheduler (Daemon)**: Headless background service (9:00 AM Scan -> 9:15-15:15 Trade -> 15:45 EOD Learn).
* **3.5 Interactive Terminal TUI / Dashboard**: Rich terminal UI showing live order book, portfolio MTM, and brain status.

---

### 📅 Week 4: Multi-Tenant REST/WebSocket Server & Broker Live Adapter
* **4.1 High-Concurrency Async API Server (Axum / Tokio)**: Handles 10,000 concurrent client connections over WebSockets & REST.
* **4.2 User Authentication & Session Management**: JWT / API key authentication with role-based rate limiting.
* **4.3 Free Indian Broker Adapter Interface**: Plug-and-play driver trait (`BrokerAdapter`) supporting Angel One SmartAPI, Dhan, and Fyers.
* **4.4 Automated TOTP 2FA Login**: Headless 9:00 AM broker login using `totp-rs` without requiring manual OTP entry.
* **4.5 Final 30-Day Forward Simulation & Hardening Checklist**: Verifying profit factor > 1.5, max drawdown < 8%, and zero race conditions.

---

## 3. What More Can We Add (Future Institutional Upgrades)

1. **Statistical Arbitrage & Pairs Trading Engine**: Z-Score mean reversion on correlated Indian pairs (e.g., HDFCBANK vs ICICIBANK).
2. **Options Greeks & Volatility Engine**: Black-Scholes Delta, Gamma, Theta, Vega for NIFTY / BANKNIFTY weekly options.
3. **Order Flow & Footprint Imbalance**: Bid-Ask volume delta analysis to spot institutional block trades.
4. **Web Frontend (React / Svelte / Leptos)**: Real-time web trading terminal for end users.
