//! ApexDesk: Scalable High-Performance Quantitative Trading Engine.

pub mod broker;
pub mod models;
pub mod orderbook;
pub mod risk;
pub mod taxes;
pub mod types;

use broker::MatchingEngine;
use models::{
    AccountId, Ledger, Order, OrderId, OrderSide, OrderType, Portfolio, TradingAccount, UserId,
};
use orderbook::OrderBook;
use risk::{CircuitBand, CircuitBreakerConfig, DailyCircuitBreaker, MarketCircuitGuard, PreTradeRiskEngine};
use types::Money;

fn main() {
    println!("⚡ ApexDesk (Rust) - Quantitative Trading Engine");
    println!("------------------------------------------------");

    let user_id = UserId(42);
    let account_id = AccountId(1001);
    let initial_capital = Money::from_rupees(100000.0);

    // 1. Double-Entry Accounting Ledger
    let mut ledger = Ledger::new();
    ledger.record_deposit(user_id, initial_capital).expect("Deposit failed");
    println!("Double-Entry Ledger Initialized (Audit: PASS)");

    // 2. Multi-Tenant Account Model
    let mut account = TradingAccount::new(account_id, user_id, initial_capital);
    println!("Account #{} (User #{}) Active - Available: {}", account.account_id.0, account.user_id.0, account.available_margin());

    // 3. Level-2 Order Book (Feature 1.8)
    let mut book = OrderBook::new("TCS.NS");
    println!("\nL2 Order Book Created for {}", book.symbol);

    // Add Bids (Buyers)
    book.add_limit_order(OrderSide::Buy, OrderId(1), UserId(10), 50, Money::from_rupees(3500.0), 100);
    book.add_limit_order(OrderSide::Buy, OrderId(2), UserId(20), 30, Money::from_rupees(3505.0), 200);

    // Add Asks (Sellers)
    book.add_limit_order(OrderSide::Sell, OrderId(3), UserId(30), 40, Money::from_rupees(3515.0), 150);
    book.add_limit_order(OrderSide::Sell, OrderId(4), UserId(40), 25, Money::from_rupees(3510.0), 250);

    let (best_bid_p, best_bid_q) = book.best_bid().unwrap();
    let (best_ask_p, best_ask_q) = book.best_ask().unwrap();
    println!("  Best Bid:  {} ({} shares)", best_bid_p, best_bid_q);
    println!("  Best Ask:  {} ({} shares)", best_ask_p, best_ask_q);
    println!("  Spread:    {}", book.spread().unwrap());
    println!("  Mid Price: {}", book.mid_price().unwrap());

    // Display Top 2 L2 Market Depth
    let (bids_l2, asks_l2) = book.l2_depth(2);
    println!("\n  [L2 Market Depth (Top 2 Levels)]");
    for (i, (p, q)) in bids_l2.iter().enumerate() {
        println!("    Bid #{}: {} | {} shares", i + 1, p, q);
    }
    for (i, (p, q)) in asks_l2.iter().enumerate() {
        println!("    Ask #{}: {} | {} shares", i + 1, p, q);
    }

    // 4. Pre-Trade Risk Engine Check (Feature 1.10)
    let tcs_entry = best_ask_p;
    let tcs_sl = Money::from_rupees(3475.0);
    let risk_engine = PreTradeRiskEngine::new();
    let max_safe_shares = risk_engine.calculate_max_shares(&account, tcs_entry, tcs_sl);
    println!("\nPre-Trade Risk Engine Sizing: Max Safe Shares = {}", max_safe_shares);

    let qty = max_safe_shares.min(7);
    let mut order = Order::new(
        OrderId(9001),
        user_id,
        account_id,
        "TCS.NS",
        OrderSide::Buy,
        OrderType::Bracket {
            entry_limit: Some(tcs_entry),
            stop_loss: tcs_sl,
            take_profit: Money::from_rupees(3580.0),
        },
        qty,
    ).unwrap();

    risk_engine.validate_order(&account, &order, tcs_entry).expect("Pre-trade risk failed");
    println!("Pre-Trade Risk Validation: PASS (1% Risk Cap & 25% Position Cap Verified)");

    // 5. Market Circuit Guard Check (Feature 1.12)
    let mut circuit_guard = MarketCircuitGuard::new();
    let tcs_circuit = circuit_guard
        .register_security(
            "TCS.NS",
            Money::from_rupees(3450.0),
            CircuitBand::TenPercent,
        )
        .clone();
    circuit_guard
        .validate_order_price("TCS.NS", OrderSide::Buy, tcs_entry)
        .expect("Circuit guard failed");
    println!("Market Circuit Guard: PASS (TCS.NS Range [LC: {}, UC: {}])",
        tcs_circuit.lower_circuit, tcs_circuit.upper_circuit);

    // 6. Matching Engine Execution (Feature 1.9)
    let order_margin = tcs_entry.mul_qty(qty);
    account.allocate_margin(order_margin).unwrap();

    let mut engine = MatchingEngine::new();
    let result = engine.match_against_book(&mut order, &mut book, 1000000);
    println!("\nMatching Engine Match Status: {:?}", result.status);
    for fill in &result.fills {
        println!("  Execution Fill #{}: {} shares @ {} (Maker: #{}, Taker: #{})",
            fill.fill_id, fill.qty, fill.price, fill.maker_order_id.0, fill.taker_order_id.0);
    }

    // 6. Portfolio Position Tracking
    let mut portfolio = Portfolio::new(user_id, account_id);
    portfolio.buy("TCS.NS", qty, tcs_entry).unwrap();
    ledger.record_position_entry(user_id, "TCS.NS", order_margin).unwrap();

    let pos = portfolio.get_position("TCS.NS").unwrap();
    println!("Portfolio Holding: {} shares @ {}", pos.qty, pos.avg_entry_price);

    // 7. Daily Loss Circuit Breaker (Feature 1.11 - User-Configurable)
    let user_custom_limit_bps = 150; // User sets 1.5% maximum daily loss limit
    let cb_config = CircuitBreakerConfig::new(user_custom_limit_bps)
        .with_consecutive_loss_limit(3);
    let mut circuit_breaker = DailyCircuitBreaker::new(cb_config);
    println!("\nUser-Configured Circuit Breaker: {} BPS ({}%) Max Daily Loss",
        user_custom_limit_bps, user_custom_limit_bps as f64 / 100.0);

    circuit_breaker.check_and_update(initial_capital, Money::zero(), portfolio.total_unrealized_pnl(), 1000000)
        .expect("Circuit breaker tripped");
    println!("Circuit Breaker Status: PASS (Active, Drawdown within limit)");
}
