//! Module: Indian Equity Intraday Transaction Charges & Taxes Calculator.
//! Accurately models NSE charges, STT, GST, Stamp Duty, SEBI fees, and Brokerage.

/// Data structure holding the complete regulatory fee breakdown for an intraday trade.
#[derive(Debug, Clone, PartialEq)]
pub struct IndianCharges {
    pub buy_turnover: f64,
    pub sell_turnover: f64,
    pub total_turnover: f64,
    pub gross_pnl: f64,
    pub brokerage: f64,
    pub stt: f64,
    pub exchange_charges: f64,
    pub gst: f64,
    pub stamp_duty: f64,
    pub sebi_charges: f64,
    pub total_taxes_and_brokerage: f64,
    pub net_pnl: f64,
}

/// Helper function to round a floating point number to 2 decimal places (paise precision).
pub fn round_to_paise(val: f64) -> f64 {
    (val * 100.0).round() / 100.0
}

/// Calculates exact Indian equity intraday regulatory charges and net P&L.
///
/// # Arguments
/// * `buy_price` - Execution price when buying a share (in Rupees)
/// * `sell_price` - Execution price when selling a share (in Rupees)
/// * `qty` - Number of shares traded
pub fn calculate_intraday_charges(buy_price: f64, sell_price: f64, qty: u32) -> IndianCharges {
    let q = qty as f64;
    let buy_turnover = round_to_paise(buy_price * q);
    let sell_turnover = round_to_paise(sell_price * q);
    let total_turnover = round_to_paise(buy_turnover + sell_turnover);

    // 1. Brokerage: ₹20 or 0.03% per executed order (whichever is lower)
    let brokerage_buy = (buy_turnover * 0.0003).min(20.0);
    let brokerage_sell = (sell_turnover * 0.0003).min(20.0);
    let brokerage = round_to_paise(brokerage_buy + brokerage_sell);

    // 2. STT (Securities Transaction Tax): 0.025% on Sell side for intraday
    let stt = round_to_paise(sell_turnover * 0.00025);

    // 3. Exchange Transaction Charges (NSE): 0.00345% of total turnover
    let exchange_charges = round_to_paise(total_turnover * 0.0000345);

    // 4. GST: 18% on (Brokerage + Exchange Transaction Charges)
    let gst = round_to_paise((brokerage + exchange_charges) * 0.18);

    // 5. Stamp Duty: 0.003% on Buy side
    let stamp_duty = round_to_paise(buy_turnover * 0.00003);

    // 6. SEBI Turnover Charges: ₹10 per crore (0.0001% of total turnover)
    let sebi_charges = round_to_paise(total_turnover * 0.000001);

    // Total regulatory friction
    let total_taxes_and_brokerage = round_to_paise(
        brokerage + stt + exchange_charges + gst + stamp_duty + sebi_charges,
    );

    let gross_pnl = round_to_paise(sell_turnover - buy_turnover);
    let net_pnl = round_to_paise(gross_pnl - total_taxes_and_brokerage);

    IndianCharges {
        buy_turnover,
        sell_turnover,
        total_turnover,
        gross_pnl,
        brokerage,
        stt,
        exchange_charges,
        gst,
        stamp_duty,
        sebi_charges,
        total_taxes_and_brokerage,
        net_pnl,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profitable_intraday_trade() {
        // Buy 100 shares at Rs 2,500 and sell at Rs 2,520 (Gross profit: Rs 2,000)
        let charges = calculate_intraday_charges(2500.0, 2520.0, 100);

        assert_eq!(charges.buy_turnover, 250000.0);
        assert_eq!(charges.sell_turnover, 252000.0);
        assert_eq!(charges.gross_pnl, 2000.0);

        // Brokerage capped at ₹20 per side = ₹40
        assert_eq!(charges.brokerage, 40.0);

        // STT = 252000 * 0.00025 = ₹63.00
        assert_eq!(charges.stt, 63.0);

        // Net PnL must be positive and strictly less than gross profit
        assert!(charges.net_pnl > 0.0);
        assert!(charges.net_pnl < charges.gross_pnl);
        assert_eq!(charges.net_pnl, 1861.36);
    }

    #[test]
    fn test_zero_quantity_edge_case() {
        let charges = calculate_intraday_charges(1000.0, 1010.0, 0);
        assert_eq!(charges.total_turnover, 0.0);
        assert_eq!(charges.gross_pnl, 0.0);
        assert_eq!(charges.net_pnl, 0.0);
    }

    #[test]
    fn test_losing_trade_increases_loss() {
        // Buy 100 shares at Rs 1,000 and sell at Rs 990 (Gross loss: Rs 1,000)
        let charges = calculate_intraday_charges(1000.0, 990.0, 100);
        assert_eq!(charges.gross_pnl, -1000.0);
        // Net loss must be larger than gross loss due to taxes
        assert!(charges.net_pnl < -1000.0);
    }
}
