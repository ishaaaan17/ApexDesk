//! Multi-Tenant Portfolio & Position Tracker.
//! Manages open equity positions, weighted-average cost basis,
//! real-time Mark-to-Market (MTM) valuations, and partial position closures.

use std::collections::HashMap;
use crate::models::account::{AccountId, UserId};
use crate::types::Money;

/// Errors returned by portfolio and position operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortfolioError {
    InsufficientShares { held: u32, requested: u32 },
    PositionNotFound(String),
    InvalidQuantity,
    InvalidPrice,
}

/// Represents an active open position in a specific stock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    pub symbol: String,
    pub qty: u32,
    pub avg_entry_price: Money,
    pub current_market_price: Money,
    pub realized_pnl: Money,
}

impl Position {
    /// Creates a new open position.
    pub fn new(symbol: impl Into<String>, qty: u32, entry_price: Money) -> Self {
        Position {
            symbol: symbol.into(),
            qty,
            avg_entry_price: entry_price,
            current_market_price: entry_price,
            realized_pnl: Money::zero(),
        }
    }

    /// Total cost basis invested in this position.
    #[inline]
    pub fn cost_basis(&self) -> Money {
        self.avg_entry_price.mul_qty(self.qty)
    }

    /// Current market value based on latest quote.
    #[inline]
    pub fn current_market_value(&self) -> Money {
        self.current_market_price.mul_qty(self.qty)
    }

    /// Real-time Mark-to-Market (MTM) unrealized profit or loss.
    #[inline]
    pub fn unrealized_pnl(&self) -> Money {
        let price_diff = self.current_market_price - self.avg_entry_price;
        price_diff.mul_qty(self.qty)
    }

    /// Percentage return on this open position (e.g. +4.5% or -1.2%).
    pub fn return_pct(&self) -> f64 {
        let cost = self.cost_basis().to_paise();
        if cost == 0 {
            return 0.0;
        }
        let pnl = self.unrealized_pnl().to_paise();
        ((pnl as f64 / cost as f64) * 10_000.0).round() / 100.0
    }

    /// Updates the current market quote and recomputes MTM valuation.
    #[inline]
    pub fn update_quote(&mut self, new_price: Money) {
        self.current_market_price = new_price;
    }

    /// Adds more shares to an existing position, re-calculating the weighted-average cost basis.
    pub fn add_shares(&mut self, additional_qty: u32, buy_price: Money) -> Result<(), PortfolioError> {
        if additional_qty == 0 {
            return Err(PortfolioError::InvalidQuantity);
        }
        let current_cost = self.cost_basis().to_paise();
        let additional_cost = buy_price.mul_qty(additional_qty).to_paise();
        let new_total_qty = self.qty + additional_qty;

        let new_avg_paise = ((current_cost + additional_cost) as f64 / new_total_qty as f64).round() as i64;

        self.qty = new_total_qty;
        self.avg_entry_price = Money::from_paise(new_avg_paise);
        self.current_market_price = buy_price;
        Ok(())
    }

    /// Reduces or closes a position, locking in realized PnL on the closed portion.
    pub fn reduce_shares(&mut self, sell_qty: u32, sell_price: Money) -> Result<Money, PortfolioError> {
        if sell_qty == 0 {
            return Err(PortfolioError::InvalidQuantity);
        }
        if sell_qty > self.qty {
            return Err(PortfolioError::InsufficientShares {
                held: self.qty,
                requested: sell_qty,
            });
        }

        let pnl_per_share = sell_price - self.avg_entry_price;
        let leg_realized_pnl = pnl_per_share.mul_qty(sell_qty);

        self.qty -= sell_qty;
        self.realized_pnl += leg_realized_pnl;
        self.current_market_price = sell_price;

        Ok(leg_realized_pnl)
    }
}

/// A user's complete multi-asset portfolio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Portfolio {
    pub user_id: UserId,
    pub account_id: AccountId,
    pub positions: HashMap<String, Position>,
}

impl Portfolio {
    /// Creates a new empty portfolio for a user account.
    pub fn new(user_id: UserId, account_id: AccountId) -> Self {
        Portfolio {
            user_id,
            account_id,
            positions: HashMap::new(),
        }
    }

    /// Returns the number of distinct stock positions currently held.
    #[inline]
    pub fn position_count(&self) -> usize {
        self.positions.len()
    }

    /// Retrieves an active position for a given symbol if it exists.
    #[inline]
    pub fn get_position(&self, symbol: &str) -> Option<&Position> {
        self.positions.get(symbol)
    }

    /// Opens a new position or accumulates into an existing one.
    pub fn buy(&mut self, symbol: &str, qty: u32, price: Money) -> Result<(), PortfolioError> {
        if qty == 0 {
            return Err(PortfolioError::InvalidQuantity);
        }
        if let Some(pos) = self.positions.get_mut(symbol) {
            pos.add_shares(qty, price)?;
        } else {
            self.positions.insert(symbol.to_string(), Position::new(symbol, qty, price));
        }
        Ok(())
    }

    /// Sells shares from a position. Removes the position if completely closed.
    /// Returns the realized gross P&L on the sold shares.
    pub fn sell(&mut self, symbol: &str, qty: u32, price: Money) -> Result<Money, PortfolioError> {
        let pos = self.positions.get_mut(symbol).ok_or_else(|| {
            PortfolioError::PositionNotFound(symbol.to_string())
        })?;

        let realized = pos.reduce_shares(qty, price)?;

        if pos.qty == 0 {
            self.positions.remove(symbol);
        }

        Ok(realized)
    }

    /// Total capital invested across all open positions.
    pub fn total_cost_basis(&self) -> Money {
        let mut total = Money::zero();
        for pos in self.positions.values() {
            total += pos.cost_basis();
        }
        total
    }

    /// Total current market value across all open positions.
    pub fn total_market_value(&self) -> Money {
        let mut total = Money::zero();
        for pos in self.positions.values() {
            total += pos.current_market_value();
        }
        total
    }

    /// Total Mark-to-Market (MTM) unrealized profit or loss across the portfolio.
    pub fn total_unrealized_pnl(&self) -> Money {
        let mut total = Money::zero();
        for pos in self.positions.values() {
            total += pos.unrealized_pnl();
        }
        total
    }

    /// Batch updates live market quotes across all open holdings.
    pub fn update_market_quotes(&mut self, quotes: &[(&str, Money)]) {
        for (sym, price) in quotes {
            if let Some(pos) = self.positions.get_mut(*sym) {
                pos.update_quote(*price);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_position_and_mtm() {
        let mut pos = Position::new("TCS.NS", 10, Money::from_rupees(3500.0));
        assert_eq!(pos.cost_basis(), Money::from_rupees(35000.0));
        assert_eq!(pos.unrealized_pnl(), Money::zero());
        assert_eq!(pos.return_pct(), 0.0);

        // Price rises to Rs 3,550 (+Rs 50/share)
        pos.update_quote(Money::from_rupees(3550.0));
        assert_eq!(pos.current_market_value(), Money::from_rupees(35500.0));
        assert_eq!(pos.unrealized_pnl(), Money::from_rupees(500.0));
        assert_eq!(pos.return_pct(), 1.43);

        // Price falls to Rs 3,450 (-Rs 50/share)
        pos.update_quote(Money::from_rupees(3450.0));
        assert_eq!(pos.unrealized_pnl(), Money::from_rupees(-500.0));
        assert_eq!(pos.return_pct(), -1.43);
    }

    #[test]
    fn test_weighted_average_cost_basis_accumulation() {
        // Buy 10 @ Rs 100, then Buy 10 @ Rs 110 -> Average entry must be Rs 105.00
        let mut pos = Position::new("RELIANCE.NS", 10, Money::from_rupees(100.0));
        pos.add_shares(10, Money::from_rupees(110.0)).unwrap();

        assert_eq!(pos.qty, 20);
        assert_eq!(pos.avg_entry_price, Money::from_rupees(105.0));
        assert_eq!(pos.cost_basis(), Money::from_rupees(2100.0));
    }

    #[test]
    fn test_partial_sale_realized_pnl() {
        let mut pos = Position::new("INFY.NS", 20, Money::from_rupees(1000.0));

        // Sell 10 shares @ Rs 1,100 -> Gain is (1100 - 1000) * 10 = Rs 1,000
        let realized = pos.reduce_shares(10, Money::from_rupees(1100.0)).unwrap();
        assert_eq!(realized, Money::from_rupees(1000.0));
        assert_eq!(pos.qty, 10);
        assert_eq!(pos.avg_entry_price, Money::from_rupees(1000.0)); // Remaining shares retain Rs 1000 cost basis
        assert_eq!(pos.realized_pnl, Money::from_rupees(1000.0));
    }

    #[test]
    fn test_oversell_rejection() {
        let mut pos = Position::new("TATAMOTORS.NS", 5, Money::from_rupees(800.0));
        let res = pos.reduce_shares(10, Money::from_rupees(850.0));

        assert_eq!(
            res,
            Err(PortfolioError::InsufficientShares {
                held: 5,
                requested: 10
            })
        );
    }

    #[test]
    fn test_portfolio_lifecycle_and_full_closure() {
        let mut portfolio = Portfolio::new(UserId(1), AccountId(101));

        // Buy 10 TCS @ Rs 3500 and 20 INFY @ Rs 1500
        portfolio.buy("TCS.NS", 10, Money::from_rupees(3500.0)).unwrap();
        portfolio.buy("INFY.NS", 20, Money::from_rupees(1500.0)).unwrap();

        assert_eq!(portfolio.position_count(), 2);
        assert_eq!(portfolio.total_cost_basis(), Money::from_rupees(65000.0)); // 35k + 30k

        // Batch update market prices
        portfolio.update_market_quotes(&[
            ("TCS.NS", Money::from_rupees(3600.0)), // +1k gain
            ("INFY.NS", Money::from_rupees(1450.0)), // -1k loss
        ]);

        assert_eq!(portfolio.total_market_value(), Money::from_rupees(65000.0));
        assert_eq!(portfolio.total_unrealized_pnl(), Money::zero());

        // Completely sell out TCS position
        let realized_tcs = portfolio.sell("TCS.NS", 10, Money::from_rupees(3650.0)).unwrap();
        assert_eq!(realized_tcs, Money::from_rupees(1500.0));
        assert_eq!(portfolio.position_count(), 1); // TCS removed from active positions
        assert!(portfolio.get_position("TCS.NS").is_none());
    }
}
