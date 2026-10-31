//! ApexDesk: Scalable High-Performance Quantitative Trading Desk.

pub mod models;
pub mod taxes;
pub mod types;

use models::{AccountId, Ledger, TradingAccount, UserId};
use types::{normalize_to_tick, BasisPoints, Money, NSE_DEFAULT_TICK_SIZE_PAISE};

fn main() {
    println!("⚡ ApexDesk (Rust) - Quantitative Trading Engine");
    println!("------------------------------------------------");

    let user_id = UserId(42);
    let initial_capital = Money::from_rupees(100000.0);

    // 1. Double-Entry Accounting Ledger (Immutable Audit Trail)
    let mut ledger = Ledger::new();
    ledger.record_deposit(user_id, initial_capital).expect("Deposit failed");
    println!("Double-Entry Ledger Initialized (Transaction #1: Deposit)");
    println!("  Audit Integrity: {}", if ledger.audit_integrity() { "PASS (Debits == Credits)" } else { "FAIL" });

    // 2. Multi-Tenant Account Model (10,000 Users Scale)
    let mut account = TradingAccount::new(AccountId(1001), user_id, initial_capital);
    println!("\nAccount #{} (User #{}) Active", account.account_id.0, account.user_id.0);
    println!("  Available Margin:   {}", account.available_margin());

    // 3. Lock Margin & Record in Ledger
    let order_margin = Money::from_rupees(25000.0);
    account.allocate_margin(order_margin).unwrap();
    ledger.record_position_entry(user_id, "TCS.NS", order_margin).unwrap();
    println!("\nLocked {} margin (Recorded in Ledger Tx #2):", order_margin);
    println!("  Allocated Margin:   {}", account.allocated_margin);
    println!("  Remaining Free:     {}", account.available_margin());

    // 4. Price Snapping & Execution
    let raw_price = Money::from_paise(251523);
    let clean_price = normalize_to_tick(raw_price, NSE_DEFAULT_TICK_SIZE_PAISE);
    let slippage_bps = BasisPoints::from_bps(5);
    let slippage_cost = slippage_bps.apply_to(clean_price.mul_qty(10));
    println!("\nExecution Simulation (10 shares):");
    println!("  Raw Price:          {}", raw_price);
    println!("  Tick-Snapped:       {}", clean_price);
    println!("  5 BPS Slippage:     {}", slippage_cost);

    // 5. Settle Trade & Record Settlement in Ledger
    let net_pnl = Money::from_paise(186136); // +Rs 1,861.36
    let fees = Money::from_paise(13864);
    let net_cash_returned = order_margin + net_pnl;

    account.release_margin(order_margin).unwrap();
    account.settle_trade(net_pnl, fees);
    ledger.record_trade_settlement(user_id, "TCS.NS", order_margin, net_cash_returned, fees, net_pnl + fees).unwrap();

    println!("\nTrade Settled & Audited (Tx #3):");
    println!("  Final Cash Balance: {}", account.cash_balance);
    println!("  Total Realized P&L: {}", account.realized_pnl);
    println!("  Ledger Tx Count:    {}", ledger.transaction_count());
    println!("  Full Ledger Audit:  {}", if ledger.audit_integrity() { "PASS 100% (Strictly Balanced)" } else { "FAIL" });
}
