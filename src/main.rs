//! ApexDesk: Scalable High-Performance Quantitative Trading Desk.

pub mod models;
pub mod taxes;
pub mod types;

use models::{AccountId, TradingAccount, UserId};
use types::{normalize_to_tick, BasisPoints, Money, NSE_DEFAULT_TICK_SIZE_PAISE};

fn main() {
    println!("⚡ ApexDesk (Rust) - Quantitative Trading Engine");
    println!("------------------------------------------------");

    // 1. Multi-Tenant Account Model (10,000 Users Scale)
    let mut account = TradingAccount::new(
        AccountId(1001),
        UserId(42),
        Money::from_rupees(100000.0), // Rs 1 Lakh initial deposit
    );
    println!("Account #{} (User #{}) Initialized", account.account_id.0, account.user_id.0);
    println!("  Total Cash Balance: {}", account.cash_balance);
    println!("  Available Margin:   {}", account.available_margin());

    // 2. Lock Margin for an Order
    let order_margin = Money::from_rupees(25000.0);
    account.allocate_margin(order_margin).expect("Failed to allocate margin");
    println!("\nLocked {} margin for incoming trade:", order_margin);
    println!("  Allocated Margin:   {}", account.allocated_margin);
    println!("  Remaining Free:     {}", account.available_margin());

    // 3. Price Snapping & Slippage
    let raw_price = Money::from_paise(251523); // Rs 2515.23
    let clean_price = normalize_to_tick(raw_price, NSE_DEFAULT_TICK_SIZE_PAISE);
    let slippage_bps = BasisPoints::from_bps(5); // 5 BPS = 0.05%
    let slippage_cost = slippage_bps.apply_to(clean_price.mul_qty(10));
    println!("\nExecution Simulation (10 shares):");
    println!("  Raw Price:          {}", raw_price);
    println!("  Tick-Snapped:       {}", clean_price);
    println!("  5 BPS Slippage:     {}", slippage_cost);

    // 4. Settle a Winning Trade (+Rs 1,861.36 net after taxes)
    let net_pnl = Money::from_paise(186136);
    let fees = Money::from_paise(13864);
    account.release_margin(order_margin).unwrap();
    account.settle_trade(net_pnl, fees);
    println!("\nTrade Settled (+{} Net P&L):", net_pnl);
    println!("  Final Cash Balance: {}", account.cash_balance);
    println!("  Total Realized P&L: {}", account.realized_pnl);
    println!("  Total Fees Paid:    {}", account.total_fees_paid);
}
