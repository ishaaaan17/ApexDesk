//! ApexDesk: Scalable High-Performance Quantitative Trading Desk.

pub mod taxes;

fn main() {
    println!("⚡ ApexDesk (Rust) - Quantitative Trading Engine");
    println!("------------------------------------------------");

    // Example calculation: Buying 50 shares of TCS at Rs 3,500 and selling at Rs 3,540
    let trade = taxes::calculate_intraday_charges(3500.0, 3540.0, 50);

    println!("Stock Trade: 50 Shares @ Rs 3,500 -> Rs 3,540");
    println!("  Gross Profit:   Rs {:>8.2}", trade.gross_pnl);
    println!("  Total Taxes:    Rs {:>8.2}", trade.total_taxes_and_brokerage);
    println!("  Net Profit:     Rs {:>8.2}", trade.net_pnl);
}
