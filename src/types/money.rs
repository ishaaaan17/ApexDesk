//! Fixed-Point Money & Currency Type.
//! Eliminates floating point rounding errors across 10,000+ concurrent accounts
//! by storing monetary values as exact 64-bit signed integers representing paise.
//! (1 Rupee = 100 Paise).

use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// Represents an exact monetary amount in Indian Rupees (INR) stored as integer paise.
/// Prevents IEEE-754 floating point inaccuracies (e.g., 0.1 + 0.2 != 0.3).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Money(pub i64);

impl Money {
    /// Creates a Money instance directly from integer paise.
    #[inline]
    pub const fn from_paise(paise: i64) -> Self {
        Money(paise)
    }

    /// Creates a Money instance from standard rupees float (rounds to nearest paise).
    pub fn from_rupees(rupees: f64) -> Self {
        Money((rupees * 100.0).round() as i64)
    }

    /// Returns the value in standard rupees as an f64 decimal.
    #[inline]
    pub fn to_rupees(&self) -> f64 {
        self.0 as f64 / 100.0
    }

    /// Returns the raw integer paise count.
    #[inline]
    pub const fn to_paise(&self) -> i64 {
        self.0
    }

    /// Creates a zero Money amount.
    #[inline]
    pub const fn zero() -> Self {
        Money(0)
    }

    /// Returns true if the monetary value is strictly positive (> 0).
    #[inline]
    pub const fn is_positive(&self) -> bool {
        self.0 > 0
    }

    /// Returns true if the monetary value is strictly negative (< 0).
    #[inline]
    pub const fn is_negative(&self) -> bool {
        self.0 < 0
    }

    /// Returns true if the monetary value is zero.
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Returns the absolute value of the monetary amount.
    #[inline]
    pub fn abs(&self) -> Self {
        Money(self.0.abs())
    }

    /// Multiplies the money amount by a whole share quantity without floating point loss.
    pub fn mul_qty(&self, qty: u32) -> Self {
        Money(self.0 * qty as i64)
    }

    /// Multiplies by a float multiplier (e.g. tax rate 0.18 for 18% GST) and rounds to paise.
    pub fn mul_rate(&self, rate: f64) -> Self {
        Money(((self.0 as f64) * rate).round() as i64)
    }

    /// Checked addition protecting against arithmetic overflow under massive load.
    pub fn checked_add(&self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Money)
    }

    /// Checked subtraction protecting against arithmetic overflow.
    pub fn checked_sub(&self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Money)
    }
}

// Operator Overload: Money + Money
impl Add for Money {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Money(self.0 + rhs.0)
    }
}

impl AddAssign for Money {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

// Operator Overload: Money - Money
impl Sub for Money {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Money(self.0 - rhs.0)
    }
}

impl SubAssign for Money {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

// Pretty print formatting: e.g. "Rs 1,250.50"
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let is_neg = self.0 < 0;
        let abs_paise = self.0.abs();
        let rupees = abs_paise / 100;
        let paise = abs_paise % 100;

        if is_neg {
            write!(f, "-Rs {}.{:02}", rupees, paise)
        } else {
            write!(f, "Rs {}.{:02}", rupees, paise)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paise_conversion_and_rounding() {
        let m = Money::from_rupees(2500.50);
        assert_eq!(m.to_paise(), 250050);
        assert_eq!(m.to_rupees(), 2500.50);
    }

    #[test]
    fn test_zero_float_drift() {
        // In IEEE-754 floats: 0.10 + 0.20 = 0.30000000000000004
        // In Money: exactly 30 paise!
        let m1 = Money::from_rupees(0.10);
        let m2 = Money::from_rupees(0.20);
        let sum = m1 + m2;
        assert_eq!(sum.to_paise(), 30);
        assert_eq!(sum.to_rupees(), 0.30);
    }

    #[test]
    fn test_multiplication_by_quantity() {
        let share_price = Money::from_rupees(350.25);
        let total_cost = share_price.mul_qty(100);
        assert_eq!(total_cost.to_paise(), 3502500);
        assert_eq!(total_cost.to_rupees(), 35025.00);
    }

    #[test]
    fn test_gst_rate_multiplication() {
        // Rs 100 brokerage with 18% GST = Rs 18
        let brokerage = Money::from_rupees(100.00);
        let gst = brokerage.mul_rate(0.18);
        assert_eq!(gst.to_rupees(), 18.00);
        assert_eq!(gst.to_paise(), 1800);
    }

    #[test]
    fn test_negative_money_formatting() {
        let loss = Money::from_rupees(-450.75);
        assert!(loss.is_negative());
        assert_eq!(format!("{}", loss), "-Rs 450.75");

        let profit = Money::from_rupees(1200.50);
        assert!(profit.is_positive());
        assert_eq!(format!("{}", profit), "Rs 1200.50");
    }

    #[test]
    fn test_checked_add_overflow_protection() {
        let max_val = Money::from_paise(i64::MAX);
        let extra = Money::from_paise(1);
        // Must safely return None instead of panicking
        assert_eq!(max_val.checked_add(extra), None);
    }
}
