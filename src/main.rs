//! ApexDesk: Scalable High-Performance Quantitative Trading Desk.

pub mod taxes;
pub mod types;

use types::Money;

fn main() {
    println!("⚡ ApexDesk (Rust) - Quantitative Trading Engine");
    println!("------------------------------------------------");

    // 1. Exact Money demonstration
    let capital = Money::from_rupees(100000.0);
    let share_price = Money::from_rupees(2515.25);
    let qty = 20;
    let order_cost = share_price.mul_qty(qty);
    let remaining_cash = capital - order_cost;

    println!("Multi-Tenant Account Capital: {}", capital);
    println!("Order: {} shares @ {} = {}", qty, share_price, order_cost);
    println!("Remaining Balance:            {}", remaining_cash);

    // 2. Exact Intraday Taxes
    let trade = taxes::calculate_intraday_charges(3500.0, 3540.0, 50);
    println!("\nTax Calculation for 50 Shares @ Rs 3500 -> Rs 3540:");
    println!("  Gross P&L: Rs {:>8.2}", trade.gross_pnl);
    println!("  Taxes:     Rs {:>8.2}", trade.total_taxes_and_brokerage);
    println!("  Net P&L:   Rs {:>8.2}", trade.net_pnl);
}
