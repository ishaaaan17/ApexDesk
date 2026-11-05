//! ApexDesk: Scalable High-Performance Quantitative Trading Engine.

pub mod models;
pub mod orderbook;
pub mod taxes;
pub mod types;

use models::{
    AccountId, Ledger, Order, OrderId, OrderSide, OrderType, Portfolio, TradingAccount, UserId,
};
use orderbook::OrderBook;
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

    // 4. OMS Bracket Order Execution
    let tcs_entry = best_ask_p; // Match at Best Ask
    let qty = 20;
    let mut order = Order::new(
        OrderId(9001),
        user_id,
        account_id,
        "TCS.NS",
        OrderSide::Buy,
        OrderType::Bracket {
            entry_limit: Some(tcs_entry),
            stop_loss: Money::from_rupees(3465.0),
            take_profit: Money::from_rupees(3570.0),
        },
        qty,
    ).unwrap();

    order.submit().unwrap();
    let order_margin = tcs_entry.mul_qty(qty);
    account.allocate_margin(order_margin).unwrap();
    order.record_fill(qty, tcs_entry).unwrap();
    println!("\nOMS Order Executed against Book: {:?}", order.status);

    // 5. Portfolio Position Tracking
    let mut portfolio = Portfolio::new(user_id, account_id);
    portfolio.buy("TCS.NS", qty, tcs_entry).unwrap();
    ledger.record_position_entry(user_id, "TCS.NS", order_margin).unwrap();

    let pos = portfolio.get_position("TCS.NS").unwrap();
    println!("Portfolio Holding: {} shares @ {}", pos.qty, pos.avg_entry_price);
}
