//! ApexDesk: Scalable High-Performance Quantitative Trading Desk.

pub mod taxes;
pub mod types;

use types::{normalize_to_tick, BasisPoints, Money, NSE_DEFAULT_TICK_SIZE_PAISE};

fn main() {
    println!("⚡ ApexDesk (Rust) - Quantitative Trading Engine");
    println!("------------------------------------------------");

    // 1. Exact Money demonstration
    let capital = Money::from_rupees(100000.0);
    let raw_price = Money::from_paise(251523); // Theoretical price Rs 2515.23
    let clean_price = normalize_to_tick(raw_price, NSE_DEFAULT_TICK_SIZE_PAISE);

    println!("Multi-Tenant Account Capital: {}", capital);
    println!("Raw Theoretical Price:        {}", raw_price);
    println!("NSE Tick-Normalized Price:    {} (Snapped to 5p grid)", clean_price);

    // 2. Basis Points (Slippage Simulation)
    let slippage_bps = BasisPoints::from_bps(5); // 5 BPS = 0.05%
    let slippage_cost = slippage_bps.apply_to(clean_price.mul_qty(50));
    println!("Simulated 5 BPS Slippage:     {}", slippage_cost);

    // 3. Exact Intraday Taxes
    let trade = taxes::calculate_intraday_charges(3500.0, 3540.0, 50);
    println!("\nTax Calculation for 50 Shares @ Rs 3500 -> Rs 3540:");
    println!("  Gross P&L: Rs {:>8.2}", trade.gross_pnl);
    println!("  Taxes:     Rs {:>8.2}", trade.total_taxes_and_brokerage);
    println!("  Net P&L:   Rs {:>8.2}", trade.net_pnl);
}
