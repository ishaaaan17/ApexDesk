//! Precision Quantitative Math Helpers, Basis Points (BPS), and Tick Size Normalizer.
//! Designed for high-frequency price snapping and exact financial ratio calculations.

use crate::types::Money;

/// Standard NSE equity tick size in India is 5 paise (Rs 0.05).
pub const NSE_DEFAULT_TICK_SIZE_PAISE: i64 = 5;

/// Basis points helper: 1 BPS = 0.01% = 0.0001 (1 part in 10,000).
/// Used across institutional finance for slippage, fees, and spread calculations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct BasisPoints(pub u32);

impl BasisPoints {
    /// Creates BasisPoints from an integer count (e.g., 50 BPS = 0.50%).
    #[inline]
    pub const fn from_bps(bps: u32) -> Self {
        BasisPoints(bps)
    }

    /// Converts a percentage (e.g. 1.5%) to basis points (150 BPS).
    pub fn from_percent(pct: f64) -> Self {
        BasisPoints((pct * 100.0).round() as u32)
    }

    /// Returns the decimal multiplier (e.g., 50 BPS -> 0.005).
    #[inline]
    pub fn to_multiplier(&self) -> f64 {
        self.0 as f64 / 10_000.0
    }

    /// Applies basis points to a monetary amount, rounding to nearest paise.
    /// E.g., 50 BPS on Rs 10,000 = Rs 50.00.
    pub fn apply_to(&self, amount: Money) -> Money {
        let raw_paise = amount.to_paise();
        let bps_paise = ((raw_paise as f64) * self.to_multiplier()).round() as i64;
        Money::from_paise(bps_paise)
    }
}

/// Normalizes an order price to the valid exchange tick size grid (default 5 paise for NSE).
/// Protects against broker rejection when theoretical models produce fractional paise prices.
///
/// # Example
/// * With a 5 paise tick size, `Rs 100.02` snaps to `Rs 100.00`, and `Rs 100.03` snaps to `Rs 100.05`.
pub fn normalize_to_tick(price: Money, tick_size_paise: i64) -> Money {
    if tick_size_paise <= 1 {
        return price;
    }
    let raw = price.to_paise();
    let remainder = raw % tick_size_paise;
    
    if remainder == 0 {
        price
    } else if remainder >= (tick_size_paise + 1) / 2 {
        Money::from_paise(raw + (tick_size_paise - remainder))
    } else {
        Money::from_paise(raw - remainder)
    }
}

/// Calculates the percentage return (P&L %) between initial capital and current balance.
/// Safely returns 0.0 if initial capital is zero to prevent division by zero.
pub fn calculate_return_pct(initial_capital: Money, current_capital: Money) -> f64 {
    let init = initial_capital.to_paise();
    if init == 0 {
        return 0.0;
    }
    let diff = current_capital.to_paise() - init;
    ((diff as f64 / init as f64) * 10000.0).round() / 100.0
}

/// Computes the exact ratio in basis points between two monetary amounts.
/// Safely returns 0 if base is zero.
pub fn calculate_bps_between(part: Money, whole: Money) -> u32 {
    let w = whole.to_paise().abs();
    if w == 0 {
        return 0;
    }
    let p = part.to_paise().abs();
    let bps = ((p as f64 / w as f64) * 10_000.0).round() as u32;
    bps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basis_points_calculation() {
        // 50 BPS (0.50%) on Rs 10,000 = Rs 50.00
        let amount = Money::from_rupees(10000.0);
        let bps = BasisPoints::from_bps(50);
        let fee = bps.apply_to(amount);
        assert_eq!(fee, Money::from_rupees(50.0));

        // 5 BPS (0.05% standard slippage) on Rs 1,00,000 = Rs 50.00 (5,000 paise)
        let turnover = Money::from_rupees(100000.0);
        let slippage_rate = BasisPoints::from_percent(0.05);
        assert_eq!(slippage_rate.apply_to(turnover), Money::from_rupees(50.0));
    }

    #[test]
    fn test_tick_size_normalization() {
        // NSE Standard 5-paise tick grid tests
        let tick = NSE_DEFAULT_TICK_SIZE_PAISE;

        // Snaps down
        let p1 = Money::from_paise(10002); // 100.02
        assert_eq!(normalize_to_tick(p1, tick), Money::from_paise(10000));

        // Snaps up
        let p2 = Money::from_paise(10003); // 100.03
        assert_eq!(normalize_to_tick(p2, tick), Money::from_paise(10005));

        let p3 = Money::from_paise(10004); // 100.04
        assert_eq!(normalize_to_tick(p3, tick), Money::from_paise(10005));

        // Exactly on grid remains unchanged
        let p4 = Money::from_paise(10005); // 100.05
        assert_eq!(normalize_to_tick(p4, tick), Money::from_paise(10005));
    }

    #[test]
    fn test_return_percentage() {
        let initial = Money::from_rupees(100000.0);
        let gain = Money::from_rupees(105500.0);
        assert_eq!(calculate_return_pct(initial, gain), 5.5);

        let loss = Money::from_rupees(98000.0);
        assert_eq!(calculate_return_pct(initial, loss), -2.0);

        // Division by zero protection
        assert_eq!(calculate_return_pct(Money::zero(), gain), 0.0);
    }

    #[test]
    fn test_bps_between_amounts() {
        let part = Money::from_rupees(25.0);
        let whole = Money::from_rupees(1000.0);
        // 25 / 1000 = 2.5% = 250 BPS
        assert_eq!(calculate_bps_between(part, whole), 250);

        // Zero safety
        assert_eq!(calculate_bps_between(part, Money::zero()), 0);
    }
}
