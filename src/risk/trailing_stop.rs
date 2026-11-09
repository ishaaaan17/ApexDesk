//! Trailing Stop-Loss Engine for ApexDesk.
//! Institutional state machine that manages dynamic stop-loss adjustments:
//! 1. Breakeven Auto-Lock (+1.0R): Moves stop to entry (+buffer) when trade reaches +1R.
//! 2. Profit Lock (+1.5R): Moves stop to lock in +1.0R profit when trade reaches +1.5R.
//! 3. Peak-Trailing: Dynamically trails behind the high-water mark as the trade expands.
//! 4. Stop-Loss Ratchet Invariant: Stop can NEVER loosen against the trader.

use crate::models::order::{OrderId, OrderSide};
use crate::types::math::{normalize_to_tick, NSE_DEFAULT_TICK_SIZE_PAISE};
use crate::types::Money;

/// Error returned when initializing an invalid trailing stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrailingStopError {
    InvalidStopDistance,
    LongStopMustBeBelowEntry { entry: Money, stop: Money },
    ShortStopMustBeAboveEntry { entry: Money, stop: Money },
}

/// User-configurable rules for the trailing stop state machine.
#[derive(Debug, Clone, PartialEq)]
pub struct TrailingConfig {
    /// R-multiple threshold to lock stop to breakeven (default: 1.0R).
    pub breakeven_trigger_r: f64,
    /// Small buffer added to breakeven stop to cover exchange fees (default: 5 paise).
    pub breakeven_buffer: Money,
    /// R-multiple threshold to lock profit (default: 1.5R).
    pub profit_lock_trigger_r: f64,
    /// Locked stop-loss R-multiple when profit lock triggers (default: 1.0R).
    pub profit_locked_r: f64,
    /// Continuous trailing distance in R-multiples behind peak price (default: 1.0R).
    pub trail_distance_r: f64,
}

impl Default for TrailingConfig {
    fn default() -> Self {
        Self {
            breakeven_trigger_r: 1.0,
            breakeven_buffer: Money::from_paise(NSE_DEFAULT_TICK_SIZE_PAISE), // 5 paise buffer
            profit_lock_trigger_r: 1.5,
            profit_locked_r: 1.0,
            trail_distance_r: 1.0,
        }
    }
}

/// Lifecycle states of a trailing stop.
#[derive(Debug, Clone, PartialEq)]
pub enum TrailingState {
    Initial,
    BreakevenLocked { locked_stop: Money },
    ProfitLocked { locked_stop: Money },
    DynamicTrailing { peak_price: Money, current_stop: Money },
    StoppedOut { exit_price: Money, realized_r: f64 },
}

/// Event emitted when a price tick updates the trailing stop state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum TrailingUpdate {
    NoChange,
    StopRatchet {
        old_stop: Money,
        new_stop: Money,
        current_r: f64,
        reason: String,
    },
    StopTriggered {
        exit_price: Money,
        stop_price: Money,
        realized_r: f64,
    },
}

/// Dynamic Trailing Stop-Loss manager for an individual trade position.
#[derive(Debug, Clone)]
pub struct TrailingStop {
    pub order_id: OrderId,
    pub symbol: String,
    pub side: OrderSide,
    pub entry_price: Money,
    pub initial_stop: Money,
    pub current_stop: Money,
    pub peak_price: Money,
    pub risk_unit_paise: i64, // 1R in paise
    pub config: TrailingConfig,
    pub state: TrailingState,
}

impl TrailingStop {
    /// Creates a new trailing stop manager for a position with default institutional configuration.
    pub fn new(
        order_id: OrderId,
        symbol: impl Into<String>,
        side: OrderSide,
        entry_price: Money,
        initial_stop: Money,
    ) -> Result<Self, TrailingStopError> {
        Self::with_config(
            order_id,
            symbol,
            side,
            entry_price,
            initial_stop,
            TrailingConfig::default(),
        )
    }

    /// Creates a new trailing stop manager with customized rules.
    pub fn with_config(
        order_id: OrderId,
        symbol: impl Into<String>,
        side: OrderSide,
        entry_price: Money,
        initial_stop: Money,
        config: TrailingConfig,
    ) -> Result<Self, TrailingStopError> {
        let risk_unit_paise = match side {
            OrderSide::Buy => {
                if initial_stop >= entry_price {
                    return Err(TrailingStopError::LongStopMustBeBelowEntry {
                        entry: entry_price,
                        stop: initial_stop,
                    });
                }
                entry_price.to_paise() - initial_stop.to_paise()
            }
            OrderSide::Sell => {
                if initial_stop <= entry_price {
                    return Err(TrailingStopError::ShortStopMustBeAboveEntry {
                        entry: entry_price,
                        stop: initial_stop,
                    });
                }
                initial_stop.to_paise() - entry_price.to_paise()
            }
        };

        if risk_unit_paise <= 0 {
            return Err(TrailingStopError::InvalidStopDistance);
        }

        let normalized_stop = normalize_to_tick(initial_stop, NSE_DEFAULT_TICK_SIZE_PAISE);

        Ok(Self {
            order_id,
            symbol: symbol.into(),
            side,
            entry_price,
            initial_stop: normalized_stop,
            current_stop: normalized_stop,
            peak_price: entry_price,
            risk_unit_paise,
            config,
            state: TrailingState::Initial,
        })
    }

    /// Calculates the current R-multiple for a given price: `(Current - Entry) / 1R`.
    #[inline]
    pub fn calculate_r(&self, current_price: Money) -> f64 {
        let gain_paise = match self.side {
            OrderSide::Buy => current_price.to_paise() - self.entry_price.to_paise(),
            OrderSide::Sell => self.entry_price.to_paise() - current_price.to_paise(),
        };
        gain_paise as f64 / self.risk_unit_paise as f64
    }

    /// Evaluates a new market quote tick and updates the stop-loss state machine.
    pub fn update_quote(&mut self, current_price: Money) -> TrailingUpdate {
        // If already stopped out, do not process further quotes
        if let TrailingState::StoppedOut { .. } = self.state {
            return TrailingUpdate::NoChange;
        }

        // 1. Check if Stop Loss was hit on this tick
        let is_stopped = match self.side {
            OrderSide::Buy => current_price <= self.current_stop,
            OrderSide::Sell => current_price >= self.current_stop,
        };

        if is_stopped {
            let realized_r = self.calculate_r(self.current_stop);
            self.state = TrailingState::StoppedOut {
                exit_price: self.current_stop,
                realized_r,
            };
            return TrailingUpdate::StopTriggered {
                exit_price: current_price,
                stop_price: self.current_stop,
                realized_r,
            };
        }

        // 2. Track the high-water mark (Peak Price)
        let is_new_peak = match self.side {
            OrderSide::Buy => current_price > self.peak_price,
            OrderSide::Sell => current_price < self.peak_price,
        };

        if is_new_peak {
            self.peak_price = current_price;
        }

        let current_r = self.calculate_r(self.peak_price);
        let old_stop = self.current_stop;

        // 3. Evaluate Trailing Rules (Breakeven -> Profit Lock -> Dynamic Trail)
        let mut candidate_stop = self.current_stop;
        let mut transition_reason = None;

        if current_r >= self.config.profit_lock_trigger_r {
            // Profit Lock Rule (+1.5R): Move stop to lock in +1.0R (or trail behind peak)
            let trail_offset_paise =
                (self.config.trail_distance_r * self.risk_unit_paise as f64).round() as i64;

            let trail_stop_paise = match self.side {
                OrderSide::Buy => self.peak_price.to_paise() - trail_offset_paise,
                OrderSide::Sell => self.peak_price.to_paise() + trail_offset_paise,
            };

            let locked_profit_paise = match self.side {
                OrderSide::Buy => {
                    self.entry_price.to_paise()
                        + (self.config.profit_locked_r * self.risk_unit_paise as f64).round() as i64
                }
                OrderSide::Sell => {
                    self.entry_price.to_paise()
                        - (self.config.profit_locked_r * self.risk_unit_paise as f64).round() as i64
                }
            };

            // Take the tighter of dynamic trail vs base profit lock
            let best_stop_paise = match self.side {
                OrderSide::Buy => trail_stop_paise.max(locked_profit_paise),
                OrderSide::Sell => trail_stop_paise.min(locked_profit_paise),
            };

            candidate_stop = normalize_to_tick(
                Money::from_paise(best_stop_paise),
                NSE_DEFAULT_TICK_SIZE_PAISE,
            );

            if current_r >= self.config.profit_lock_trigger_r + 0.5 {
                self.state = TrailingState::DynamicTrailing {
                    peak_price: self.peak_price,
                    current_stop: candidate_stop,
                };
                transition_reason = Some(format!(
                    "Dynamic Trailing (Peak: {}, Target Trail: {}R)",
                    self.peak_price, self.config.trail_distance_r
                ));
            } else {
                self.state = TrailingState::ProfitLocked {
                    locked_stop: candidate_stop,
                };
                transition_reason = Some(format!(
                    "Profit Lock Triggered (+{:.2}R Peak, Locked at +{:.2}R)",
                    current_r, self.config.profit_locked_r
                ));
            }
        } else if current_r >= self.config.breakeven_trigger_r {
            // Breakeven Lock Rule (+1.0R): Move stop to Entry (+ buffer)
            let breakeven_stop = match self.side {
                OrderSide::Buy => self.entry_price + self.config.breakeven_buffer,
                OrderSide::Sell => self.entry_price - self.config.breakeven_buffer,
            };

            candidate_stop =
                normalize_to_tick(breakeven_stop, NSE_DEFAULT_TICK_SIZE_PAISE);
            self.state = TrailingState::BreakevenLocked {
                locked_stop: candidate_stop,
            };
            transition_reason = Some(format!(
                "Breakeven Auto-Lock Triggered (+{:.2}R reached)",
                current_r
            ));
        }

        // 4. Strict Stop Ratchet Invariant: Stop can NEVER loosen against the trader
        let should_ratchet = match self.side {
            OrderSide::Buy => candidate_stop > self.current_stop,
            OrderSide::Sell => candidate_stop < self.current_stop,
        };

        if should_ratchet {
            self.current_stop = candidate_stop;
            TrailingUpdate::StopRatchet {
                old_stop,
                new_stop: self.current_stop,
                current_r,
                reason: transition_reason.unwrap_or_else(|| "Stop Ratchet".to_string()),
            }
        } else {
            TrailingUpdate::NoChange
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_breakeven_auto_lock_at_1r() {
        // Buy Reliance @ Rs 2,500.00 with Stop @ Rs 2,450.00 (1R = Rs 50.00)
        let mut ts = TrailingStop::new(
            OrderId(1),
            "RELIANCE.NS",
            OrderSide::Buy,
            Money::from_rupees(2500.0),
            Money::from_rupees(2450.0),
        )
        .unwrap();

        assert_eq!(ts.risk_unit_paise, 5000); // Rs 50 = 5000 paise
        assert_eq!(ts.current_stop, Money::from_rupees(2450.0));

        // Price rises to Rs 2,525.00 (+0.5R) -> No change to stop
        let update = ts.update_quote(Money::from_rupees(2525.0));
        assert_eq!(update, TrailingUpdate::NoChange);
        assert_eq!(ts.current_stop, Money::from_rupees(2450.0));

        // Price reaches Rs 2,550.00 (+1.0R) -> Breakeven Auto-Lock fires!
        // Stop moves to Rs 2,500.05 (Entry + 5p buffer)
        let update = ts.update_quote(Money::from_rupees(2550.0));
        match update {
            TrailingUpdate::StopRatchet { old_stop, new_stop, current_r, .. } => {
                assert_eq!(old_stop, Money::from_rupees(2450.0));
                assert_eq!(new_stop, Money::from_rupees(2500.05));
                assert_eq!(current_r, 1.0);
            }
            _ => panic!("Expected StopRatchet event"),
        }
        assert_eq!(ts.current_stop, Money::from_rupees(2500.05));
    }

    #[test]
    fn test_long_profit_lock_at_1_5r() {
        let mut ts = TrailingStop::new(
            OrderId(2),
            "RELIANCE.NS",
            OrderSide::Buy,
            Money::from_rupees(2500.0),
            Money::from_rupees(2450.0), // 1R = Rs 50.00
        )
        .unwrap();

        // Jump to Rs 2,575.00 (+1.5R) -> Profit Lock fires!
        // Stop moves to Entry + 1.0R = Rs 2,550.00
        let update = ts.update_quote(Money::from_rupees(2575.0));
        match update {
            TrailingUpdate::StopRatchet { old_stop, new_stop, current_r, .. } => {
                assert_eq!(old_stop, Money::from_rupees(2450.0));
                assert_eq!(new_stop, Money::from_rupees(2550.0));
                assert_eq!(current_r, 1.5);
            }
            _ => panic!("Expected StopRatchet"),
        }
        assert_eq!(ts.current_stop, Money::from_rupees(2550.0));
    }

    #[test]
    fn test_long_dynamic_peak_trailing_and_stopped_out() {
        let mut ts = TrailingStop::new(
            OrderId(3),
            "RELIANCE.NS",
            OrderSide::Buy,
            Money::from_rupees(2500.0),
            Money::from_rupees(2450.0), // 1R = Rs 50.00
        )
        .unwrap();

        // Price climbs to Rs 2,650.00 (+3.0R)
        // Trailing distance = 1.0R (Rs 50.00) behind peak -> Stop ratchets to Rs 2,600.00 (+2.0R)
        let _ = ts.update_quote(Money::from_rupees(2650.0));
        assert_eq!(ts.current_stop, Money::from_rupees(2600.0));

        // Price pulls back slightly to Rs 2,620.00 -> Stop does NOT loosen (stays Rs 2,600.00)
        let update = ts.update_quote(Money::from_rupees(2620.0));
        assert_eq!(update, TrailingUpdate::NoChange);
        assert_eq!(ts.current_stop, Money::from_rupees(2600.0));

        // Price drops to Rs 2,595.00 -> Hits stop at Rs 2,600.00!
        let update = ts.update_quote(Money::from_rupees(2595.0));
        match update {
            TrailingUpdate::StopTriggered { exit_price, stop_price, realized_r } => {
                assert_eq!(exit_price, Money::from_rupees(2595.0));
                assert_eq!(stop_price, Money::from_rupees(2600.0));
                assert_eq!(realized_r, 2.0); // Locked in +2.0R profit!
            }
            _ => panic!("Expected StopTriggered"),
        }
    }

    #[test]
    fn test_short_breakeven_and_profit_lock() {
        // Short TCS @ Rs 3,500.00 with Stop @ Rs 3,550.00 (1R = Rs 50.00)
        let mut ts = TrailingStop::new(
            OrderId(4),
            "TCS.NS",
            OrderSide::Sell,
            Money::from_rupees(3500.0),
            Money::from_rupees(3550.0),
        )
        .unwrap();

        // Price drops to Rs 3,450.00 (+1.0R gain) -> Breakeven fires!
        // Stop ratchets down to Rs 3,499.95 (Entry - 5p buffer)
        let update = ts.update_quote(Money::from_rupees(3450.0));
        match update {
            TrailingUpdate::StopRatchet { new_stop, current_r, .. } => {
                assert_eq!(new_stop, Money::from_rupees(3499.95));
                assert_eq!(current_r, 1.0);
            }
            _ => panic!("Expected Breakeven Ratchet"),
        }

        // Price drops to Rs 3,425.00 (+1.5R gain) -> Profit Lock fires!
        // Stop ratchets down to Rs 3,450.00 (+1.0R profit locked)
        let update = ts.update_quote(Money::from_rupees(3425.0));
        match update {
            TrailingUpdate::StopRatchet { new_stop, current_r, .. } => {
                assert_eq!(new_stop, Money::from_rupees(3450.0));
                assert_eq!(current_r, 1.5);
            }
            _ => panic!("Expected Profit Lock Ratchet"),
        }
    }

    #[test]
    fn test_invalid_initial_stop_rejected() {
        // Long with stop above entry -> Error
        let err = TrailingStop::new(
            OrderId(5),
            "INFY.NS",
            OrderSide::Buy,
            Money::from_rupees(1500.0),
            Money::from_rupees(1550.0),
        );
        assert!(matches!(
            err,
            Err(TrailingStopError::LongStopMustBeBelowEntry { .. })
        ));

        // Short with stop below entry -> Error
        let err = TrailingStop::new(
            OrderId(6),
            "INFY.NS",
            OrderSide::Sell,
            Money::from_rupees(1500.0),
            Money::from_rupees(1450.0),
        );
        assert!(matches!(
            err,
            Err(TrailingStopError::ShortStopMustBeAboveEntry { .. })
        ));
    }
}
