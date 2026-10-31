//! Comprehensive End-to-End Integration Test for Pillars 1 & 2.
//! Simulates multiple concurrent Indian traders, price snapping, margin locking,
//! realistic regulatory taxation, trade settlement, and double-entry ledger auditing.

use apex_desk::models::{AccountId, Ledger, TradingAccount, UserId};
use apex_desk::taxes::calculate_intraday_charges;
use apex_desk::types::{
    calculate_return_pct, normalize_to_tick, BasisPoints, Money, NSE_DEFAULT_TICK_SIZE_PAISE,
};

#[test]
fn test_end_to_end_multi_user_trading_and_accounting_lifecycle() {
    println!("\n==================================================================");
    println!("  RUNNING COMPREHENSIVE COMBINED INTEGRATION TEST (PILLARS 1 & 2) ");
    println!("==================================================================");

    // 1. Initialize Global Double-Entry Ledger (Audit Log for all users)
    let mut global_ledger = Ledger::new();

    // -------------------------------------------------------------
    // USER 1: Profitable Intraday Scalper (TCS.NS)
    // -------------------------------------------------------------
    let user1_id = UserId(101);
    let user1_account_id = AccountId(1001);
    let user1_capital = Money::from_rupees(100000.0); // Rs 1,00,000

    let mut user1_account = TradingAccount::new(user1_account_id, user1_id, user1_capital);
    global_ledger.record_deposit(user1_id, user1_capital).unwrap();

    // Stock: TCS.NS - Raw model calculates entry at Rs 3512.23
    let raw_entry_price = Money::from_paise(351223);
    // Snaps to valid NSE 5-paise tick grid -> Rs 3512.25
    let clean_entry_price = normalize_to_tick(raw_entry_price, NSE_DEFAULT_TICK_SIZE_PAISE);
    assert_eq!(clean_entry_price, Money::from_rupees(3512.25));

    let shares_qty: u32 = 25;
    let required_margin = clean_entry_price.mul_qty(shares_qty); // Rs 87,806.25
    assert_eq!(required_margin, Money::from_rupees(87806.25));

    // Lock margin & record in double-entry ledger
    user1_account.allocate_margin(required_margin).unwrap();
    global_ledger.record_position_entry(user1_id, "TCS.NS", required_margin).unwrap();

    assert_eq!(user1_account.available_margin(), Money::from_rupees(12193.75));

    // Target Hit at Rs 3550.00!
    let clean_exit_price = Money::from_rupees(3550.00);

    // Calculate exact Indian regulatory taxes & fees
    let tax_report = calculate_intraday_charges(
        clean_entry_price.to_rupees(),
        clean_exit_price.to_rupees(),
        shares_qty,
    );

    // Apply 5 BPS execution slippage simulation
    let slippage_bps = BasisPoints::from_bps(5);
    let slippage = slippage_bps.apply_to(clean_exit_price.mul_qty(shares_qty));

    let gross_profit = Money::from_rupees(tax_report.gross_pnl);
    let total_taxes = Money::from_rupees(tax_report.total_taxes_and_brokerage);
    let net_profit = Money::from_rupees(tax_report.net_pnl) - slippage;

    assert!(net_profit.is_positive());
    assert_eq!(gross_profit, Money::from_rupees(943.75)); // 25 shares * Rs 37.75 gain

    // Settle Trade: Release margin, credit profit, and post balanced ledger entries
    user1_account.release_margin(required_margin).unwrap();
    user1_account.settle_trade(net_profit, total_taxes + slippage);

    let net_cash_returned = required_margin + net_profit;
    global_ledger
        .record_trade_settlement(
            user1_id,
            "TCS.NS",
            required_margin,
            net_cash_returned,
            total_taxes + slippage,
            gross_profit,
        )
        .unwrap();

    assert_eq!(user1_account.allocated_margin, Money::zero());
    assert_eq!(user1_account.realized_pnl, net_profit);
    assert!(user1_account.cash_balance > user1_capital);

    let user1_return_pct = calculate_return_pct(user1_capital, user1_account.cash_balance);
    println!("  [+] User 1 Completed Trade: Net P&L {}, Return: +{:.2}%", net_profit, user1_return_pct);

    // -------------------------------------------------------------
    // USER 2: Losing Intraday Trade (INFY.NS - Stop Loss Triggered)
    // -------------------------------------------------------------
    let user2_id = UserId(102);
    let user2_account_id = AccountId(1002);
    let user2_capital = Money::from_rupees(50000.0); // Rs 50,000

    let mut user2_account = TradingAccount::new(user2_account_id, user2_id, user2_capital);
    global_ledger.record_deposit(user2_id, user2_capital).unwrap();

    // Buy 20 shares of INFY at Rs 1,500.00 = Rs 30,000 margin
    let infy_entry = Money::from_rupees(1500.00);
    let infy_qty: u32 = 20;
    let infy_margin = infy_entry.mul_qty(infy_qty);

    user2_account.allocate_margin(infy_margin).unwrap();
    global_ledger.record_position_entry(user2_id, "INFY.NS", infy_margin).unwrap();

    // Stop Loss Hit at Rs 1,485.00 (-Rs 15/share = -Rs 300 gross loss)
    let infy_exit = Money::from_rupees(1485.00);
    let user2_tax = calculate_intraday_charges(infy_entry.to_rupees(), infy_exit.to_rupees(), infy_qty);

    let user2_gross_loss = Money::from_rupees(user2_tax.gross_pnl);
    let user2_fees = Money::from_rupees(user2_tax.total_taxes_and_brokerage);
    let user2_net_loss = Money::from_rupees(user2_tax.net_pnl);

    assert_eq!(user2_gross_loss, Money::from_rupees(-300.00));
    assert!(user2_net_loss < user2_gross_loss); // Net loss strictly greater due to taxes!

    user2_account.release_margin(infy_margin).unwrap();
    user2_account.settle_trade(user2_net_loss, user2_fees);

    let user2_cash_returned = infy_margin + user2_net_loss;
    global_ledger
        .record_trade_settlement(
            user2_id,
            "INFY.NS",
            infy_margin,
            user2_cash_returned,
            user2_fees,
            user2_gross_loss,
        )
        .unwrap();

    assert_eq!(user2_account.allocated_margin, Money::zero());
    assert_eq!(user2_account.cash_balance, user2_capital + user2_net_loss);

    let user2_return_pct = calculate_return_pct(user2_capital, user2_account.cash_balance);
    println!("  [-] User 2 Stopped Out: Net P&L {}, Return: {:.2}%", user2_net_loss, user2_return_pct);

    // -------------------------------------------------------------
    // GLOBAL DOUBLE-ENTRY AUDIT ACROSS ALL USERS
    // -------------------------------------------------------------
    println!("\n  [*] Auditing Global Multi-Tenant Ledger across all user transactions...");
    assert_eq!(global_ledger.transaction_count(), 6); // 2 deposits + 2 opens + 2 settlements
    assert!(global_ledger.audit_integrity(), "GLOBAL DOUBLE-ENTRY AUDIT FAILED!");

    println!("  [SUCCESS] All Debits strictly equal Credits down to the exact paise.");
    println!("==================================================================\n");
}
