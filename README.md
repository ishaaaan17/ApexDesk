# ApexDesk ⚡

> **High-Performance Scalable Quantitative Trading Desk & Automated Engine**  
> *Architected for High-Concurrency, Real-Time Market Execution, and Test-Driven Development (TDD)*

Developed by **[Ishaan Maheshwari (@ishaaaan17)](https://github.com/ishaaaan17)**

---

## 🏛️ Project Vision & Standards

* **Test-Driven Development (TDD)**: Every single feature starts with Unit Tests, followed by Integration Tests, and System-Level Acceptance Tests.
* **Granular Micro-Commits**: Strictly one feature per commit with clean, atomic history.
* **10,000+ Concurrent User Scalability**: Multi-tenant architecture, non-blocking asynchronous I/O, and thread-safe data persistence.
* **100% Free Public Market Data**: Zero paid API dependencies for educational & simulation environments.

---

## 📅 Module 1: The Institutional Quant Core (18 Micro-Features Roadmap)

### Pillar 1: Financial Precision & Math
* [x] **1.1 Indian Regulatory Tax Engine** (`src/taxes.rs`): STT, GST, SEBI, Stamp Duty, NSE Exchange charges, Brokerage ₹20 precision calculation.
* [ ] **1.2 Fixed-Point Money & Currency Type** (`src/types/money.rs`): Paise-precision, zero float-rounding bugs for ledger accuracy.
* [ ] **1.3 Precision Rounding & Math Helpers** (`src/types/math.rs`): Fast fixed-point arithmetic, basis point multipliers, and decimal normalization.

### Pillar 2: Multi-Tenant Accounts & Audit (10,000 Users Scale)
* [ ] **1.4 Multi-Tenant User & Account State Model** (`src/models/account.rs`): Thread-safe isolated account balances.
* [ ] **1.5 Double-Entry Accounting Ledger** (`src/models/ledger.rs`): Immutable audit trail where every debit matches a credit.
* [ ] **1.6 Portfolio & Position Tracker** (`src/models/portfolio.rs`): Real-time Mark-to-Market (MTM) & FIFO cost accounting.

### Pillar 3: Order Management System (OMS)
* [ ] **1.7 Bracket, Market, Limit & Stop Order State Machine** (`src/models/order.rs`): Order lifecycle and state transitions.
* [ ] **1.8 In-Memory Order Book** (`src/orderbook/mod.rs`): Price-Time priority matching queue.
* [ ] **1.9 High-Throughput Matching Engine** (`src/broker/matching_engine.rs`): Simulates real fills, partial fills, and slippage.

### Pillar 4: Pre-Trade Risk & Circuit Breakers
* [ ] **1.10 Pre-Trade Risk Engine** (`src/risk/pre_trade.rs`): 1% max risk cap per trade and margin verification.
* [ ] **1.11 Daily Loss Circuit Breaker & Emergency Kill-Switch** (`src/risk/circuit_breaker.rs`): Panic button & daily loss limits.
* [ ] **1.12 Indian Market Upper/Lower Circuit Guard** (`src/risk/market_circuits.rs`): 5%, 10%, 20% circuit band protection.
* [ ] **1.13 Trailing Stop-Loss Engine** (`src/risk/trailing_stop.rs`): Breakeven auto-lock (+1R) and trailing profit lock (+1.5R).

### Pillar 5: Market Data & Quant Math Core
* [ ] **1.14 Multi-Timeframe Candle Aggregator** (`src/indicators/bars.rs`): Aggregates ticks to 1m, 5m, 15m OHLCV bars.
* [ ] **1.15 Opening Range Breakout (ORB) Engine** (`src/indicators/orb.rs`): 15m/30m High/Low/Mid breakout calculation.
* [ ] **1.16 Quant Math Engine** (`src/indicators/math.rs`): 20/50 DMA, 14-period ATR, and Volume Weighted Average Price (VWAP).

### Pillar 6: Verification & Benchmarks (10,000 Users Scale)
* [ ] **1.17 Multi-Threaded Stress Test** (`tests/concurrency_test.rs`): Simulates 1,000+ simultaneous virtual orders.
* [ ] **1.18 Sub-Microsecond Performance Benchmarks** (`benches/core_bench.rs`): Nanosecond benchmarking with `cargo bench`.
