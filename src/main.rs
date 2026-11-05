//! ApexDesk: Scalable High-Performance Quantitative Trading Desk.

pub mod models;
pub mod taxes;
pub mod types;

use models::{AccountId, Ledger, Portfolio, TradingAccount, UserId};
use types::{normalize_to_tick, Money, NSE_DEFAULT_TICK_SIZE_PAISE};

fn main() {
    println!("⚡ ApexDesk (Rust) - Quantitative Trading Engine");
    println!("------------------------------------------------");

    let user_id = UserId(42);
    let account_id = AccountId(1001);
    let initial_capital = Money::from_rupees(100000.0);

    // 1. Double-Entry Accounting Ledger
    let mut ledger = Ledger::new();
    ledger.record_deposit(user_id, initial_capital).expect("Deposit failed");
    println!("Double-Entry Ledger Initialized (Audit: {})", if ledger.audit_integrity() { "PASS" } else { "FAIL" });

    // 2. Multi-Tenant Account Model
    let mut account = TradingAccount::new(account_id, user_id, initial_capital);
    println!("Account #{} (User #{}) Active - Available: {}", account.account_id.0, account.user_id.0, account.available_margin());

    // 3. Portfolio & Position Tracker (Feature 1.6)
    let mut portfolio = Portfolio::new(user_id, account_id);
    let tcs_entry = Money::from_rupees(3500.0);
    let tcs_qty = 20;
    let tcs_margin = tcs_entry.mul_qty(tcs_qty); // Rs 70,000

    account.allocate_margin(tcs_margin).unwrap();
    portfolio.buy("TCS.NS", tcs_qty, tcs_entry).unwrap();
    ledger.record_position_entry(user_id, "TCS.NS", tcs_margin).unwrap();

    println!("\nPortfolio Holdings (1 Position Open):");
    let pos = portfolio.get_position("TCS.NS").unwrap();
    println!("  Holding: {} shares of {} @ {}", pos.qty, pos.symbol, pos.avg_entry_price);
    println!("  Cost Basis:         {}", pos.cost_basis());
    println!("  Unrealized MTM P&L: {} ({:+.2}%)", pos.unrealized_pnl(), pos.return_pct());

    // 4. Price Fluctuation (Mark-to-Market Update)
    let new_quote = Money::from_rupees(3545.00); // Rallies +Rs 45/share
    portfolio.update_market_quotes(&[("TCS.NS", new_quote)]);
    let pos_updated = portfolio.get_position("TCS.NS").unwrap();
    println!("\nLive Quote Update -> TCS.NS at Rs 3545.00:");
    println!("  Current MTM Value:  {}", pos_updated.current_market_value());
    println!("  Unrealized MTM P&L: {} ({:+.2}%)", pos_updated.unrealized_pnl(), pos_updated.return_pct());
    println!("  Portfolio Total MTM:{}", portfolio.total_unrealized_pnl());

    // 5. Partial Profit Exit (Sell 10 shares @ Rs 3550)
    let exit_price = normalize_to_tick(Money::from_paise(355000), NSE_DEFAULT_TICK_SIZE_PAISE);
    let realized_gain = portfolio.sell("TCS.NS", 10, exit_price).unwrap();
    println!("\nPartial Exit (Sold 10 shares @ {}):", exit_price);
    println!("  Gross Realized Gain:{}", realized_gain);
    println!("  Remaining Holding:  {} shares of TCS.NS", portfolio.get_position("TCS.NS").unwrap().qty);
}
