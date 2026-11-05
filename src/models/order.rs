//! Order Management System (OMS) - Order Types and State Machine.
//! Supports Market, Limit, StopLoss, and institutional 3-legged Bracket Orders
//! with strict lifecycle state transitions, partial fills, and overfill guards.

use crate::models::account::{AccountId, UserId};
use crate::types::Money;

/// Unique identifier for an order.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderId(pub u64);

/// Order direction: Buy (Long) or Sell (Short / Exit).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order Time-In-Force (TIF) condition.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum TimeInForce {
    #[default]
    Day,
    Ioc, // Immediate-or-Cancel
    Gtc, // Good-till-Cancelled
}

/// Specific order execution types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderType {
    Market,
    Limit { limit_price: Money },
    StopLoss { stop_price: Money },
    Bracket {
        entry_limit: Option<Money>,
        stop_loss: Money,
        take_profit: Money,
    },
}

/// Strict lifecycle states of an order in the OMS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderStatus {
    Pending,
    Submitted,
    PartiallyFilled {
        filled_qty: u32,
        remaining_qty: u32,
        avg_fill_price: Money,
    },
    Filled {
        filled_qty: u32,
        avg_fill_price: Money,
    },
    Cancelled {
        reason: String,
    },
    Rejected {
        reason: String,
    },
}

/// Errors returned by the order state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderError {
    InvalidStateTransition { current: String, action: String },
    OverfillAttempt { remaining: u32, requested: u32 },
    InvalidQuantity,
    InvalidPrice(String),
}

/// Complete representation of an order in the ApexDesk OMS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub order_id: OrderId,
    pub user_id: UserId,
    pub account_id: AccountId,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub qty: u32,
    pub status: OrderStatus,
    pub filled_qty: u32,
    pub cumulative_fill_cost: Money,
}

impl Order {
    /// Creates a new market or limit order in Pending state.
    pub fn new(
        order_id: OrderId,
        user_id: UserId,
        account_id: AccountId,
        symbol: impl Into<String>,
        side: OrderSide,
        order_type: OrderType,
        qty: u32,
    ) -> Result<Self, OrderError> {
        if qty == 0 {
            return Err(OrderError::InvalidQuantity);
        }

        // Validate bracket parameters if applicable
        if let OrderType::Bracket {
            entry_limit,
            stop_loss,
            take_profit,
        } = &order_type
        {
            if let Some(entry) = entry_limit {
                if side == OrderSide::Buy {
                    if *stop_loss >= *entry {
                        return Err(OrderError::InvalidPrice(
                            "Bracket Stop Loss must be strictly below Entry Price for Long orders"
                                .to_string(),
                        ));
                    }
                    if *take_profit <= *entry {
                        return Err(OrderError::InvalidPrice(
                            "Bracket Take Profit must be strictly above Entry Price for Long orders"
                                .to_string(),
                        ));
                    }
                }
            }
        }

        Ok(Order {
            order_id,
            user_id,
            account_id,
            symbol: symbol.into(),
            side,
            order_type,
            time_in_force: TimeInForce::Day,
            qty,
            status: OrderStatus::Pending,
            filled_qty: 0,
            cumulative_fill_cost: Money::zero(),
        })
    }

    /// Transitions order from Pending to Submitted (sent to matching engine / broker).
    pub fn submit(&mut self) -> Result<(), OrderError> {
        match self.status {
            OrderStatus::Pending => {
                self.status = OrderStatus::Submitted;
                Ok(())
            }
            _ => Err(OrderError::InvalidStateTransition {
                current: format!("{:?}", self.status),
                action: "submit".to_string(),
            }),
        }
    }

    /// Records an execution fill (full or partial) with weighted-average fill price.
    pub fn record_fill(&mut self, fill_qty: u32, fill_price: Money) -> Result<&OrderStatus, OrderError> {
        if fill_qty == 0 {
            return Err(OrderError::InvalidQuantity);
        }

        let remaining = self.qty - self.filled_qty;
        if fill_qty > remaining {
            return Err(OrderError::OverfillAttempt {
                remaining,
                requested: fill_qty,
            });
        }

        match self.status {
            OrderStatus::Submitted | OrderStatus::PartiallyFilled { .. } => {
                self.filled_qty += fill_qty;
                self.cumulative_fill_cost += fill_price.mul_qty(fill_qty);

                let avg_paise = ((self.cumulative_fill_cost.to_paise() as f64)
                    / (self.filled_qty as f64))
                    .round() as i64;
                let avg_fill_price = Money::from_paise(avg_paise);

                if self.filled_qty == self.qty {
                    self.status = OrderStatus::Filled {
                        filled_qty: self.filled_qty,
                        avg_fill_price,
                    };
                } else {
                    self.status = OrderStatus::PartiallyFilled {
                        filled_qty: self.filled_qty,
                        remaining_qty: self.qty - self.filled_qty,
                        avg_fill_price,
                    };
                }
                Ok(&self.status)
            }
            _ => Err(OrderError::InvalidStateTransition {
                current: format!("{:?}", self.status),
                action: "fill".to_string(),
            }),
        }
    }

    /// Cancels an order if it is in an active cancellable state.
    pub fn cancel(&mut self, reason: impl Into<String>) -> Result<(), OrderError> {
        match self.status {
            OrderStatus::Pending | OrderStatus::Submitted | OrderStatus::PartiallyFilled { .. } => {
                self.status = OrderStatus::Cancelled {
                    reason: reason.into(),
                };
                Ok(())
            }
            _ => Err(OrderError::InvalidStateTransition {
                current: format!("{:?}", self.status),
                action: "cancel".to_string(),
            }),
        }
    }

    /// Rejects an order during risk pre-validation.
    pub fn reject(&mut self, reason: impl Into<String>) -> Result<(), OrderError> {
        match self.status {
            OrderStatus::Pending | OrderStatus::Submitted => {
                self.status = OrderStatus::Rejected {
                    reason: reason.into(),
                };
                Ok(())
            }
            _ => Err(OrderError::InvalidStateTransition {
                current: format!("{:?}", self.status),
                action: "reject".to_string(),
            }),
        }
    }

    /// Returns true if the order has reached a terminal state (cannot be filled or modified).
    #[inline]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Filled { .. } | OrderStatus::Cancelled { .. } | OrderStatus::Rejected { .. }
        )
    }

    /// Returns remaining unfilled share count.
    #[inline]
    pub fn remaining_qty(&self) -> u32 {
        self.qty - self.filled_qty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_order() -> Order {
        Order::new(
            OrderId(1),
            UserId(42),
            AccountId(1001),
            "RELIANCE.NS",
            OrderSide::Buy,
            OrderType::Limit {
                limit_price: Money::from_rupees(2500.0),
            },
            100, // 100 shares
        )
        .unwrap()
    }

    #[test]
    fn test_order_creation_initial_state() {
        let order = setup_test_order();
        assert_eq!(order.status, OrderStatus::Pending);
        assert_eq!(order.qty, 100);
        assert_eq!(order.filled_qty, 0);
        assert_eq!(order.remaining_qty(), 100);
        assert!(!order.is_terminal());
    }

    #[test]
    fn test_order_submission_and_full_fill() {
        let mut order = setup_test_order();

        // Pending -> Submitted
        order.submit().unwrap();
        assert_eq!(order.status, OrderStatus::Submitted);

        // Fill all 100 shares at Rs 2,498.00
        order.record_fill(100, Money::from_rupees(2498.0)).unwrap();

        assert_eq!(order.filled_qty, 100);
        assert_eq!(order.remaining_qty(), 0);
        assert!(order.is_terminal());

        if let OrderStatus::Filled {
            filled_qty,
            avg_fill_price,
        } = order.status
        {
            assert_eq!(filled_qty, 100);
            assert_eq!(avg_fill_price, Money::from_rupees(2498.0));
        } else {
            panic!("Expected OrderStatus::Filled");
        }
    }

    #[test]
    fn test_partial_fills_with_weighted_average_price() {
        let mut order = setup_test_order();
        order.submit().unwrap();

        // 1st Fill: 40 shares @ Rs 2,500
        order.record_fill(40, Money::from_rupees(2500.0)).unwrap();
        assert_eq!(order.filled_qty, 40);
        assert_eq!(order.remaining_qty(), 60);

        // 2nd Fill: 60 shares @ Rs 2,510
        // Weighted Average = (40*2500 + 60*2510) / 100 = (100000 + 150600) / 100 = Rs 2506.00
        order.record_fill(60, Money::from_rupees(2510.0)).unwrap();
        assert_eq!(order.filled_qty, 100);
        assert_eq!(order.remaining_qty(), 0);

        if let OrderStatus::Filled {
            filled_qty,
            avg_fill_price,
        } = order.status
        {
            assert_eq!(filled_qty, 100);
            assert_eq!(avg_fill_price, Money::from_rupees(2506.0));
        } else {
            panic!("Expected OrderStatus::Filled");
        }
    }

    #[test]
    fn test_overfill_rejection() {
        let mut order = setup_test_order();
        order.submit().unwrap();

        // Attempting to fill 105 shares on a 100-share order must be rejected
        let res = order.record_fill(105, Money::from_rupees(2500.0));
        assert_eq!(
            res,
            Err(OrderError::OverfillAttempt {
                remaining: 100,
                requested: 105
            })
        );
    }

    #[test]
    fn test_cannot_cancel_already_filled_order() {
        let mut order = setup_test_order();
        order.submit().unwrap();
        order.record_fill(100, Money::from_rupees(2500.0)).unwrap();

        // Cancelling a filled order must return InvalidStateTransition error
        let res = order.cancel("User changed mind");
        assert!(res.is_err());
    }

    #[test]
    fn test_valid_bracket_order_validation() {
        // Valid long bracket: Entry Rs 1000, Stop Loss Rs 990, Take Profit Rs 1020
        let bracket = Order::new(
            OrderId(2),
            UserId(1),
            AccountId(101),
            "TCS.NS",
            OrderSide::Buy,
            OrderType::Bracket {
                entry_limit: Some(Money::from_rupees(1000.0)),
                stop_loss: Money::from_rupees(990.0),
                take_profit: Money::from_rupees(1020.0),
            },
            50,
        );
        assert!(bracket.is_ok());

        // Invalid long bracket: Stop Loss >= Entry must be rejected!
        let bad_bracket = Order::new(
            OrderId(3),
            UserId(1),
            AccountId(101),
            "TCS.NS",
            OrderSide::Buy,
            OrderType::Bracket {
                entry_limit: Some(Money::from_rupees(1000.0)),
                stop_loss: Money::from_rupees(1005.0), // Invalid SL!
                take_profit: Money::from_rupees(1020.0),
            },
            50,
        );
        assert!(bad_bracket.is_err());
    }
}
