# ApexDesk: Master Engineering Specification & Architecture Blueprint ⚡

> **High-Performance Scalable Quantitative Trading Desk & Automated Engine**  
> *Target Scale: 10,000+ Concurrent Users • 100% Free Public Market Data • 100% Rust Architecture*  
> **Author & Maintainer:** Ishaan Maheshwari ([@ishaaaan17](https://github.com/ishaaaan17))  
> **Repository:** [https://github.com/ishaaaan17/ApexDesk](https://github.com/ishaaaan17/ApexDesk)  
> **Status:** Active Development (30-Day Engineering Sprint)

---

## 🧭 TABLE OF CONTENTS
1. [The "WHY": Philosophy & Technical Rationale](#1-the-why-philosophy--technical-rationale)
2. [The "WHAT": Complete System Capabilities](#2-the-what-complete-system-capabilities)
3. [The "HOW": System Architecture & Data Flow](#3-the-how-system-architecture--data-flow)
4. [Scalability Blueprint (10,000+ Users)](#4-scalability-blueprint-10000-users)
5. [The Indian Market Nuances & Risk Defenses](#5-the-indian-market-nuances--risk-defenses)
6. [The 30-Day Master Engineering Roadmap](#6-the-30-day-master-engineering-roadmap)
7. [Engineering Protocols & Git Discipline](#7-engineering-protocols--git-discipline)

---

## 1. The "WHY": Philosophy & Technical Rationale

### 1.1 Why build ApexDesk?
95% of retail day traders lose money because of **emotional bias**, **lack of risk management**, and **revenge trading**. Quantitative trading desks and hedge funds win not by predicting the future, but by executing cold, mathematical expected-value ($EV$) strategies with strict risk constraints:

$$\text{Expected Value } (EV) = (P_{\text{win}} \times \text{Avg Win}) - (P_{\text{loss}} \times \text{Avg Loss}) - \text{Friction (Taxes + Slippage)}$$

ApexDesk is engineered to eliminate human emotional interference completely. Every trade decision is backed by mathematical probability, historical Monte Carlo simulations, and automatic capital protection.

---

### 1.2 Why 100% Rust?
* **Zero Runtime Crashes (Compile-Time Type Safety):** In Python or JavaScript, a live trade can blow up in production due to a runtime `NoneType has no attribute 'price'` error. Rust eliminates this at compile time.
* **Zero Garbage Collection Stutters:** Java, C#, and Python pause execution periodically to clean up memory (GC pauses of 10ms - 50ms). In trading, a 20ms freeze can cause massive slippage. Rust uses compile-time ownership with **0 GC pauses**.
* **Sub-Microsecond Mathematical Throughput:** Rust compiles directly to native x86_64 machine code, executing financial calculations, order book matching, and tax breakdowns in under 1 microsecond.
* **Single Standalone Executable:** Zero dependency conflicts, virtualenv nightmares, or Python runtime mismatches. Compiles into a single portable binary (`apex_desk.exe`).

---

### 1.3 Why Paper Trading with EOD Learning First?
You never risk a single real rupee until a quantitative model has proven statistical significance in forward testing. ApexDesk runs on **real live NSE tick data** using **virtual imaginary capital (₹1,00,000)** while deducting 100% realistic Indian taxes and slippage. 

Only when the system achieves:
1. **Profit Factor > 1.5** (Gross Profit / Gross Loss)
2. **Win Rate > 50%** (with 1:2 Risk-to-Reward)
3. **Max Drawdown < 8%** across at least 30 to 50 market sessions,  
do we switch the flag from `PAPER` to `LIVE`.

---

### 1.4 Why Fixed-Point `Money` (Paise) instead of Float `f64`?
In standard IEEE-754 floating-point arithmetic:
$$0.10 + 0.20 = 0.30000000000000004$$
Over 10,000 users executing millions of transactions, fractional sub-penny drifts corrupt accounting ledgers. In ApexDesk, **all money is stored as a 64-bit integer representing exact Indian Paise** (₹1.00 = 100 Paise). Floating point math is strictly forbidden in the ledger.

---

## 2. The "WHAT": Complete System Capabilities

### 2.1 The 11 Quant Day-Trader Skills
ApexDesk incorporates 11 specialized trading desk modules:
1. **Chart Scanner**: Scans liquid NSE stocks for 15-minute Opening Range Breakouts (ORB) and 20-day high momentum.
2. **News Monitor**: Pulls real-time catalysts, news headlines, and regulatory filings.
3. **Entry Timer**: Bar-by-bar price action watcher that triggers strictly when a 5-minute candle closes above the opening range high.
4. **Exit Planner**: Automatically computes the Stop Loss (at range midpoint) and Profit Target (at 2.0R) before entry.
5. **Time Machine (Monte Carlo)**: Replays 10,000 simulated sessions from the stock's own past 60-day candles to verify empirical win probability before taking a trade.
6. **Budget Manager**: Fractional position sizing strictly fitting the 1% risk limit.
7. **Risk Guard**: Enforces non-bypassable pre-trade validation rules.
8. **Paper Broker**: High-throughput execution engine simulating order fills, fees, and portfolio balances.
9. **Price Alerts**: Real-time threshold notifications on targets, stops, and 3:15 PM EOD square-off.
10. **Earnings Analyst**: Analyzes quarterly 10-Q/8-K financial filings and corporate releases.
11. **Insider / Disclosure Tracker**: Tracks corporate bulk/block deals and insider disclosures.

---

### 2.2 Multi-Factor Quantitative Screener (Composite Score 0 to 100)
Every stock in the watchlist is evaluated across 5 quantitative pillars:
1. **Trend & Alignment (35 Points):** Price > 20 DMA, Price > 50 DMA, and 20 DMA > 50 DMA (Golden Trend).
2. **Breakout Proximity (25 Points):** Distance to 20-Day High (within 1% = highest breakout probability).
3. **Institutional Volume Footprint (20 Points):** Volume surge > 1.2x 20-day Average Daily Volume (ADV).
4. **Volatility Efficiency (10 Points):** 14-period Average True Range (ATR %) suitability (1.5% - 4.0% daily range).
5. **AI Learned Multiplier (10 Points):** Dynamic historical win-rate multiplier from the Reinforcement Learning Brain.

---

### 2.3 Trailing Stop-Loss State Machine
Protects capital and ensures a winning trade never turns into a loss:
* **Trigger 1 (+1.0R Profit):** Stop-loss is automatically shifted to **Entry Price (Breakeven)**. The trade becomes 100% risk-free.
* **Trigger 2 (+1.5R Profit):** Stop-loss is automatically trailed to **+0.75R Profit**, locking in guaranteed gains.
* **Trigger 3 (+2.0R Target):** Position closes in full profit.

---

## 3. The "HOW": System Architecture & Data Flow

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        INDIAN STOCK MARKET (NSE / BSE)                                 │
│             100% Free Live Public Feed (RELIANCE, TCS, INFY, HDFCBANK...)              │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        LAYER 1: MARKET DATA & QUANT AGGREGATOR                         │
│   • Multi-timeframe OHLCV bar aggregation (1m, 5m, 15m)                                │
│   • 15-Minute Opening Range calculation (High, Low, Midpoint)                          │
│   • Quantitative Math: 20 DMA, 50 DMA, 200 DMA, 14-period ATR, VWAP                    │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                     LAYER 2: MULTI-FACTOR QUANTITATIVE SCREENER                        │
│   • Scores every watchlist stock from 0 to 100 Points                                  │
│   • Categorizes into: TOP_PICK, ACCUMULATE_ON_DIPS, NEUTRAL_WATCH, AVOID               │
│   • Generates recommended portfolio allocation %                                       │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                   LAYER 3: PRE-TRADE RISK ENGINE & CIRCUIT GUARDS                      │
│   • 1% Account Risk Cap per Trade                                                      │
│   • Indian Market Upper/Lower Circuit Breaker (5%, 10%, 20% bands)                     │
│   • Daily Max Drawdown Hard Stop (2% daily loss = auto script freeze)                  │
│   • Trailing Stop-Loss State Machine (+1R Breakeven Lock, +1.5R Profit Lock)           │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│               LAYER 4: HIGH-THROUGHPUT EXECUTION & MATCHING ENGINE                     │
│   • In-Memory L2 Price-Time Priority Order Book                                        │
│   • Simulated Slippage Modeling (0.03% - 0.05%)                                        │
│   • Exact Indian Regulatory Taxes (STT, GST, SEBI fee, Stamp duty, ₹20 Brokerage)      │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│            LAYER 5: MULTI-TENANT LEDGER & PORTFOLIO (10,000+ Users)                    │
│   • Multi-Tenant User Isolation (`user_id`, `account_id`)                              │
│   • Thread-Safe Double-Entry Bookkeeping (`debit == credit`)                           │
│   • FIFO Position Tracking & Real-Time Mark-to-Market (MTM) P&L                        │
│   • Fixed-Point `Money` Type (Zero floating point rounding drift)                      │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│               LAYER 6: CONTINUOUS REINFORCEMENT LEARNING BRAIN                         │
│   • Immutable Trade Journal (Records every fill, fee, R-multiple, duration)            │
│   • End-of-Day (EOD) Walk-Forward Policy Optimization                                  │
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

## 4. Scalability Blueprint (10,000+ Users)

A system designed for 10,000 users cannot use flat JSON files or blocking file locks. 

### 4.1 Data Isolation (Multi-Tenancy)
* Every portfolio, balance, and order is keyed by an isolated `user_id` (`UUIDv4`).
* State access is managed via non-blocking asynchronous concurrency (`tokio::sync::RwLock`).

### 4.2 Double-Entry Accounting
Every monetary movement consists of two balancing entries:
$$\sum \text{Debits} = \sum \text{Credits}$$
* Buying 10 shares of Reliance:
  * **Debit:** `Asset:Positions:RELIANCE` (₹25,000)
  * **Credit:** `Asset:Cash` (₹25,000)
* This guarantees that account balances can never become corrupted or artificially inflated.

### 4.3 High-Throughput Matching
The in-memory matching engine executes thousands of simulated fills per millisecond, decoupling price evaluation from network I/O.

---

## 5. The Indian Market Nuances & Risk Defenses

1. **Upper / Lower Circuit Breaker Defense:**  
   Indian stocks have hard circuit limits (5%, 10%, 20%). If a stock hits a lower circuit, liquidity disappears and sell orders freeze. ApexDesk checks price distance to circuit limits and automatically blocks any entry within 2% of a circuit band.
2. **Quarterly Results Blackout Filter:**  
   On corporate earnings announcement days, bid-ask spreads widen and gap risk explodes. ApexDesk automatically flags and bans stocks on their quarterly results date.
3. **NIFTY 50 Market Regime Filter:**  
   A rising tide lifts all boats, and a market dump sinks them. If NIFTY 50 is down -1.0% or trading below its VWAP, long breakout signals on individual stocks are strictly suppressed.
4. **Mandatory 3:15 PM IST Auto Square-Off:**  
   Intraday MIS positions held past 3:20 PM incur auto-square-off penalty charges by Indian brokers. ApexDesk enforces hard client-side closure at 3:15 PM IST.

---

## 6. The 30-Day Master Engineering Roadmap

### 📅 Week 1: The Institutional Quant Core (18 Micro-Features)
* [x] **1.1 Indian Regulatory Tax Engine** (`src/taxes.rs`)
* [x] **1.2 Fixed-Point Money & Currency Type** (`src/types/money.rs`)
* [x] **1.3 Precision Rounding & Math Helpers** (`src/types/math.rs`)
* [ ] **1.4 Multi-Tenant User & Account State Model** (`src/models/account.rs`)
* [ ] **1.5 Double-Entry Accounting Ledger** (`src/models/ledger.rs`)
* [ ] **1.6 Portfolio & Position Tracker** (`src/models/portfolio.rs`)
* [ ] **1.7 Bracket, Market, Limit & Stop Order State Machine** (`src/models/order.rs`)
* [ ] **1.8 In-Memory Order Book** (`src/orderbook/mod.rs`)
* [ ] **1.9 High-Throughput Matching Engine** (`src/broker/matching_engine.rs`)
* [ ] **1.10 Pre-Trade Risk Engine** (`src/risk/pre_trade.rs`)
* [ ] **1.11 Daily Loss Circuit Breaker & Kill-Switch** (`src/risk/circuit_breaker.rs`)
* [ ] **1.12 Indian Market Upper/Lower Circuit Guard** (`src/risk/market_circuits.rs`)
* [ ] **1.13 Trailing Stop-Loss Engine** (`src/risk/trailing_stop.rs`)
* [ ] **1.14 Multi-Timeframe Candle Aggregator** (`src/indicators/bars.rs`)
* [ ] **1.15 Opening Range Breakout (ORB) Engine** (`src/indicators/orb.rs`)
* [ ] **1.16 Quant Math Engine** (`src/indicators/math.rs`)
* [ ] **1.17 Multi-Threaded Stress Test** (`tests/concurrency_test.rs`)
* [ ] **1.18 Sub-Microsecond Performance Benchmarks** (`benches/core_bench.rs`)

### 📅 Week 2: Advanced Pre-Trade Risk & Market Intelligence
* **2.1 NIFTY 50 Market Regime Filter**
* **2.2 Quarterly Earnings Event Blackout Filter**
* **2.3 Multi-Timeframe Re-Test Confirmation Engine** (15m ORB + 5m retest)
* **2.4 Volatility Squeeze Detector** (Bollinger Bands + Keltner Channels)
* **2.5 Automated EOD Square-Off Daemon** (Strict 3:15 PM IST)

### 📅 Week 3: Market Data Feeds, Telegram Alerts & Scheduler
* **3.1 100% Free Live NSE Tick Streamer**
* **3.2 Custom Watchlist Manager** (`trader/custom_watchlist.txt`)
* **3.3 Free Telegram Notification Bot** (Instant phone alerts)
* **3.4 Windows Background Scheduler (Daemon)** (9:00 AM - 3:45 PM IST automation)
* **3.5 Interactive Terminal TUI / Dashboard**

### 📅 Week 4: Multi-Tenant REST/WebSocket Server & Broker Live Adapter
* **4.1 High-Concurrency Async API Server (Axum / Tokio)**
* **4.2 User Authentication & Session Management**
* **4.3 Free Indian Broker Adapter Interface** (Angel One, Dhan, Fyers)
* **4.4 Automated TOTP 2FA Login** (Headless 9:00 AM login)
* **4.5 Final 30-Day Forward Simulation & Hardening Checklist**

---

## 7. Engineering Protocols & Git Discipline

To guarantee uncompromised software quality throughout the 30-day journey:

1. **Test-Driven Development (TDD):** Every feature must have unit tests verifying both standard operations and edge cases before being committed.
2. **One Feature = One Git Commit:** No bundling of unrelated changes. Commits follow the Conventional Commits standard (`feat:`, `fix:`, `docs:`, `test:`).
3. **Continuous GitHub Synchronization:** Every atomic commit is immediately pushed to `origin main` at [github.com/ishaaaan17/ApexDesk](https://github.com/ishaaaan17/ApexDesk).
4. **Daily Rhythm:** 2 to 3 micro-features per day, reviewed and authorized by the maintainer.
