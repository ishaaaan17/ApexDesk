//! Daily Loss Circuit Breaker and Emergency Panic Kill-Switch for ApexDesk.
//! Protects client capital by monitoring real-time intraday drawdown (realized + unrealized MTM)
//! against user-defined loss limits, with an atomic panic shutdown procedure.

use std::collections::HashMap;

use crate::models::account::UserId;
use crate::models::order::OrderSide;
use crate::models::portfolio::Portfolio;
use crate::orderbook::OrderBook;
use crate::types::math::BasisPoints;
use crate::types::Money;

/// Reason why the circuit breaker was tripped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TripReason {
    DailyLossLimitExceeded { loss: Money, limit: Money },
    RupeeLossLimitExceeded { loss: Money, limit: Money },
    ConsecutiveLossLimitExceeded { count: u32, limit: u32 },
}

/// Operational state of the risk circuit breaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitState {
    Active,
    Halted {
        reason: TripReason,
        timestamp_ns: u64,
    },
    EmergencyKilled {
        reason: String,
        timestamp_ns: u64,
    },
}

/// User-configurable circuit breaker settings.
/// Limits can be set as Basis Points of capital (e.g. 150 BPS = 1.5%, 200 BPS = 2.0%)
/// or as an absolute Rupee amount, giving the trader complete customization.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Maximum daily drawdown in Basis Points (e.g., 100 = 1.0%, 200 = 2.0%, 500 = 5.0%).
    pub max_daily_loss_bps: u32,
    /// Optional absolute Rupee loss ceiling (e.g., Rs 5,000 max daily loss).
    pub max_daily_loss_rupees: Option<Money>,
    /// Optional consecutive losing trades trip limit (e.g., halt after 3 straight losses).
    pub max_consecutive_losses: Option<u32>,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            max_daily_loss_bps: 200, // Default 2.0% daily loss limit if unconfigured
            max_daily_loss_rupees: None,
            max_consecutive_losses: None,
        }
    }
}

impl CircuitBreakerConfig {
    /// Creates a customized circuit breaker configuration specifying the maximum daily loss in Basis Points.
    /// E.g. `CircuitBreakerConfig::new(150)` sets a 1.5% daily drawdown threshold.
    pub fn new(max_daily_loss_bps: u32) -> Self {
        Self {
            max_daily_loss_bps,
            max_daily_loss_rupees: None,
            max_consecutive_losses: None,
        }
    }

    /// Sets an absolute Rupee daily loss ceiling (e.g. stop trading if losing more than Rs 2,500 today).
    pub fn with_rupee_loss_limit(mut self, limit: Money) -> Self {
        self.max_daily_loss_rupees = Some(limit);
        self
    }

    /// Sets a streak breaker: halts if consecutive losing trades reach the threshold.
    pub fn with_consecutive_loss_limit(mut self, count: u32) -> Self {
        self.max_consecutive_losses = Some(count);
        self
    }
}

/// Result summary returned by the emergency panic kill-switch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanicSummary {
    pub orders_cancelled: usize,
    pub positions_liquidated: usize,
    pub total_liquidated_value: Money,
}

/// The intraday risk circuit breaker.
#[derive(Debug, Clone)]
pub struct DailyCircuitBreaker {
    pub config: CircuitBreakerConfig,
    pub state: CircuitState,
    pub consecutive_losses: u32,
}

impl DailyCircuitBreaker {
    /// Creates a circuit breaker with user-defined configuration.
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitState::Active,
            consecutive_losses: 0,
        }
    }

    /// Returns true if the circuit breaker is active and permits trading.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.state == CircuitState::Active
    }

    /// Returns the current state of the circuit breaker.
    #[inline]
    pub fn state(&self) -> &CircuitState {
        &self.state
    }

    /// Evaluates current intraday P&L (realized + unrealized) against the user's daily limits.
    /// Trips into `Halted` mode if limits are breached.
    pub fn check_and_update(
        &mut self,
        starting_capital: Money,
        realized_pnl: Money,
        unrealized_pnl: Money,
        timestamp_ns: u64,
    ) -> Result<(), TripReason> {
        if let CircuitState::Halted { reason, .. } = &self.state {
            return Err(reason.clone());
        }
        if let CircuitState::EmergencyKilled { .. } = &self.state {
            return Err(TripReason::DailyLossLimitExceeded {
                loss: Money::zero(),
                limit: Money::zero(),
            });
        }

        let total_pnl = realized_pnl + unrealized_pnl;

        if total_pnl.is_negative() {
            let current_loss = Money::from_paise(-total_pnl.to_paise());

            // 1. Check Basis Points Drawdown Cap
            let bps_limit =
                BasisPoints(self.config.max_daily_loss_bps).apply_to(starting_capital);
            if current_loss > bps_limit {
                let reason = TripReason::DailyLossLimitExceeded {
                    loss: current_loss,
                    limit: bps_limit,
                };
                self.state = CircuitState::Halted {
                    reason: reason.clone(),
                    timestamp_ns,
                };
                return Err(reason);
            }

            // 2. Check Rupee Loss Ceiling (if configured by user)
            if let Some(rupee_limit) = self.config.max_daily_loss_rupees {
                if current_loss > rupee_limit {
                    let reason = TripReason::RupeeLossLimitExceeded {
                        loss: current_loss,
                        limit: rupee_limit,
                    };
                    self.state = CircuitState::Halted {
                        reason: reason.clone(),
                        timestamp_ns,
                    };
                    return Err(reason);
                }
            }
        }

        Ok(())
    }

    /// Records the outcome of a closed trade to track consecutive loss streaks.
    pub fn record_trade_outcome(&mut self, pnl: Money, timestamp_ns: u64) -> Result<(), TripReason> {
        if pnl.is_negative() {
            self.consecutive_losses += 1;
            if let Some(limit) = self.config.max_consecutive_losses {
                if self.consecutive_losses >= limit {
                    let reason = TripReason::ConsecutiveLossLimitExceeded {
                        count: self.consecutive_losses,
                        limit,
                    };
                    self.state = CircuitState::Halted {
                        reason: reason.clone(),
                        timestamp_ns,
                    };
                    return Err(reason);
                }
            }
        } else if pnl.is_positive() {
            self.consecutive_losses = 0;
        }

        Ok(())
    }

    /// Atomic Panic Kill-Switch (`--kill-all`):
    /// 1. Immediately transitions the circuit breaker into `EmergencyKilled`.
    /// 2. Cancels all resting open orders across all active order books.
    /// 3. Flattens and liquidates all open portfolio positions at current market quotes.
    pub fn panic_kill_switch(
        &mut self,
        user_id: UserId,
        portfolio: &mut Portfolio,
        orderbooks: &mut HashMap<String, OrderBook>,
        timestamp_ns: u64,
        reason: impl Into<String>,
    ) -> PanicSummary {
        let reason_str = reason.into();
        self.state = CircuitState::EmergencyKilled {
            reason: reason_str,
            timestamp_ns,
        };

        // 1. Cancel all resting orders in order books
        let mut cancelled_count = 0;
        for book in orderbooks.values_mut() {
            // Cancel resting bids
            let user_bids: Vec<_> = book
                .bids_mut()
                .values()
                .flat_map(|q| q.iter())
                .filter(|o| o.user_id == user_id)
                .map(|o| o.order_id)
                .collect();

            for order_id in user_bids {
                if book.cancel_order(OrderSide::Buy, order_id) {
                    cancelled_count += 1;
                }
            }

            // Cancel resting asks
            let user_asks: Vec<_> = book
                .asks_mut()
                .values()
                .flat_map(|q| q.iter())
                .filter(|o| o.user_id == user_id)
                .map(|o| o.order_id)
                .collect();

            for order_id in user_asks {
                if book.cancel_order(OrderSide::Sell, order_id) {
                    cancelled_count += 1;
                }
            }
        }

        // 2. Liquidate all open portfolio positions
        let symbols: Vec<String> = portfolio.positions.keys().cloned().collect();
        let mut liquidated_count = 0;
        let mut total_value = Money::zero();

        for symbol in symbols {
            if let Some(pos) = portfolio.positions.get(&symbol) {
                let qty = pos.qty;
                let exit_price = pos.current_market_price;
                total_value += exit_price.mul_qty(qty);
                let _ = portfolio.sell(&symbol, qty, exit_price);
                liquidated_count += 1;
            }
        }

        PanicSummary {
            orders_cancelled: cancelled_count,
            positions_liquidated: liquidated_count,
            total_liquidated_value: total_value,
        }
    }

    /// Admin / supervisor manual reset after audit review.
    pub fn reset(&mut self) {
        self.state = CircuitState::Active;
        self.consecutive_losses = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::account::AccountId;
    use crate::models::order::OrderId;

    #[test]
    fn test_user_configurable_daily_loss_bps() {
        // User configures 150 BPS (1.5%) on a Rs 100,000 account -> Max loss Rs 1,500
        let config = CircuitBreakerConfig::new(150);
        let mut cb = DailyCircuitBreaker::new(config);

        let starting_capital = Money::from_rupees(100_000.0);

        // Rs 1,000 loss (within 1.5% limit) -> PASS
        let res = cb.check_and_update(
            starting_capital,
            Money::from_rupees(-1000.0),
            Money::zero(),
            1000,
        );
        assert!(res.is_ok());
        assert!(cb.is_active());

        // Rs 1,600 loss (exceeds Rs 1,500 limit) -> TRIPS
        let res = cb.check_and_update(
            starting_capital,
            Money::from_rupees(-1600.0),
            Money::zero(),
            2000,
        );
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            TripReason::DailyLossLimitExceeded {
                loss: Money::from_rupees(1600.0),
                limit: Money::from_rupees(1500.0),
            }
        );
        assert!(!cb.is_active());
    }

    #[test]
    fn test_user_configurable_rupee_loss_limit() {
        // User configures flat Rs 2,500 daily loss limit
        let config = CircuitBreakerConfig::new(500) // 5% BPS limit = Rs 5,000
            .with_rupee_loss_limit(Money::from_rupees(2500.0)); // tighter flat rupee ceiling
        let mut cb = DailyCircuitBreaker::new(config);

        let starting_capital = Money::from_rupees(100_000.0);

        // Rs 2,600 loss exceeds the Rs 2,500 flat ceiling
        let res = cb.check_and_update(
            starting_capital,
            Money::from_rupees(-2600.0),
            Money::zero(),
            1000,
        );
        assert_eq!(
            res.unwrap_err(),
            TripReason::RupeeLossLimitExceeded {
                loss: Money::from_rupees(2600.0),
                limit: Money::from_rupees(2500.0),
            }
        );
    }

    #[test]
    fn test_consecutive_losses_trip_and_reset() {
        let config = CircuitBreakerConfig::new(300).with_consecutive_loss_limit(3);
        let mut cb = DailyCircuitBreaker::new(config);

        // 2 consecutive losses -> ok
        assert!(cb.record_trade_outcome(Money::from_rupees(-200.0), 100).is_ok());
        assert!(cb.record_trade_outcome(Money::from_rupees(-150.0), 200).is_ok());
        assert_eq!(cb.consecutive_losses, 2);

        // 1 winning trade -> streak resets to 0
        assert!(cb.record_trade_outcome(Money::from_rupees(400.0), 300).is_ok());
        assert_eq!(cb.consecutive_losses, 0);

        // 3 consecutive losses -> trips
        assert!(cb.record_trade_outcome(Money::from_rupees(-100.0), 400).is_ok());
        assert!(cb.record_trade_outcome(Money::from_rupees(-100.0), 500).is_ok());
        let res = cb.record_trade_outcome(Money::from_rupees(-100.0), 600);

        assert_eq!(
            res.unwrap_err(),
            TripReason::ConsecutiveLossLimitExceeded { count: 3, limit: 3 }
        );
        assert!(!cb.is_active());
    }

    #[test]
    fn test_panic_kill_switch_cancels_orders_and_flattens_positions() {
        let user_id = UserId(42);
        let account_id = AccountId(1001);
        let mut cb = DailyCircuitBreaker::new(CircuitBreakerConfig::default());

        // Setup Portfolio with 10 shares of RELIANCE @ Rs 2500
        let mut portfolio = Portfolio::new(user_id, account_id);
        portfolio
            .buy("RELIANCE.NS", 10, Money::from_rupees(2500.0))
            .unwrap();

        // Setup OrderBook with 1 resting order for user 42
        let mut orderbooks = HashMap::new();
        let mut book = OrderBook::new("RELIANCE.NS");
        book.add_limit_order(
            OrderSide::Buy,
            OrderId(999),
            user_id,
            15,
            Money::from_rupees(2490.0),
            1000,
        );
        orderbooks.insert("RELIANCE.NS".to_string(), book);

        // Trigger Emergency Panic Kill-Switch
        let summary = cb.panic_kill_switch(
            user_id,
            &mut portfolio,
            &mut orderbooks,
            5000,
            "Market Flash Crash Emergency",
        );

        assert_eq!(summary.orders_cancelled, 1);
        assert_eq!(summary.positions_liquidated, 1);
        assert_eq!(summary.total_liquidated_value, Money::from_rupees(25_000.0));

        // Portfolio positions must now be completely flattened (empty)
        assert_eq!(portfolio.position_count(), 0);

        // Order book must have 0 orders for user 42
        let book = orderbooks.get("RELIANCE.NS").unwrap();
        assert_eq!(book.total_bid_volume(), 0);

        // Circuit breaker state must be EmergencyKilled
        assert!(matches!(cb.state(), CircuitState::EmergencyKilled { .. }));
    }

    #[test]
    fn test_manual_admin_reset() {
        let mut cb = DailyCircuitBreaker::new(CircuitBreakerConfig::new(100));
        let _ = cb.check_and_update(
            Money::from_rupees(100_000.0),
            Money::from_rupees(-2000.0),
            Money::zero(),
            1000,
        );
        assert!(!cb.is_active());

        // Admin resets
        cb.reset();
        assert!(cb.is_active());
    }
}
