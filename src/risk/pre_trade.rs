//! Pre-Trade Risk Engine for ApexDesk.
//! Enforces non-bypassable institutional pre-trade risk controls:
//! 1. 1% Account Risk Cap per Trade (Maximum allowable loss at Stop Loss)
//! 2. Maximum Single Position Exposure Cap (e.g. 25% of account equity)
//! 3. Available Margin Verification
//! 4. Account Status Invariant (Suspended/Closed accounts blocked)
//! 5. Fat-finger order quantity limits
//! 6. Surveillance / Blacklisted symbol blocking
//! 7. Mathematical position sizing calculator

use std::collections::HashSet;

use crate::models::account::{AccountStatus, TradingAccount};
use crate::models::order::{Order, OrderSide, OrderType};
use crate::types::math::BasisPoints;
use crate::types::Money;

/// Reasons why an order is rejected by the pre-trade risk engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskRejection {
    AccountNotActive {
        current_status: AccountStatus,
    },
    InsufficientMargin {
        required: Money,
        available: Money,
    },
    RiskExceedsMaxAccountRisk {
        risk_amount: Money,
        max_allowed: Money,
    },
    MaxPositionExposureExceeded {
        position_value: Money,
        max_allowed: Money,
    },
    MaxOrderQuantityExceeded {
        requested_qty: u32,
        max_qty: u32,
    },
    SymbolBlocked {
        symbol: String,
    },
    InvalidStopLoss(String),
}

/// Risk parameters and limits configured for an account or desk.
#[derive(Debug, Clone)]
pub struct RiskLimits {
    /// Maximum risk per trade in Basis Points of total balance (e.g. 100 BPS = 1.0%).
    pub max_risk_bps: u32,
    /// Maximum single position exposure in Basis Points of total balance (e.g. 2500 BPS = 25.0%).
    pub max_position_bps: u32,
    /// Maximum quantity in a single order to prevent fat-finger entry.
    pub max_order_qty: u32,
    /// Set of blocked / surveillance symbols (e.g. ASM/GSM list, illiquid penny stocks).
    pub blocked_symbols: HashSet<String>,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_risk_bps: 100,      // 1.0% maximum risk of account balance
            max_position_bps: 2500, // 25.0% maximum position exposure
            max_order_qty: 10_000,
            blocked_symbols: HashSet::new(),
        }
    }
}

/// The non-bypassable pre-trade risk engine.
#[derive(Debug, Clone)]
pub struct PreTradeRiskEngine {
    pub limits: RiskLimits,
}

impl PreTradeRiskEngine {
    /// Creates a new risk engine with default institutional risk limits (1% risk cap, 25% position cap).
    pub fn new() -> Self {
        Self {
            limits: RiskLimits::default(),
        }
    }

    /// Creates a custom risk engine with specified limits.
    pub fn with_limits(limits: RiskLimits) -> Self {
        Self { limits }
    }

    /// Blocks a specific symbol from trading (e.g., ASM/GSM list, corporate actions).
    pub fn block_symbol(&mut self, symbol: impl Into<String>) {
        self.limits.blocked_symbols.insert(symbol.into());
    }

    /// Unblocks a previously blocked symbol.
    pub fn unblock_symbol(&mut self, symbol: &str) {
        self.limits.blocked_symbols.remove(symbol);
    }

    /// Validates an incoming order against all institutional pre-trade risk invariants.
    ///
    /// # Parameters
    /// - `account`: The trading account placing the order.
    /// - `order`: The order to validate.
    /// - `estimated_price`: Current market or quote price (used if order is a Market order).
    pub fn validate_order(
        &self,
        account: &TradingAccount,
        order: &Order,
        estimated_price: Money,
    ) -> Result<(), RiskRejection> {
        // 1. Account Status Check
        if account.status != AccountStatus::Active {
            return Err(RiskRejection::AccountNotActive {
                current_status: account.status.clone(),
            });
        }

        // 2. Blocked / Surveillance Symbol Check
        if self.limits.blocked_symbols.contains(&order.symbol) {
            return Err(RiskRejection::SymbolBlocked {
                symbol: order.symbol.clone(),
            });
        }

        // 3. Fat-Finger Maximum Quantity Check
        if order.qty > self.limits.max_order_qty {
            return Err(RiskRejection::MaxOrderQuantityExceeded {
                requested_qty: order.qty,
                max_qty: self.limits.max_order_qty,
            });
        }

        // Determine effective order price
        let effective_price = match &order.order_type {
            OrderType::Limit { limit_price } => *limit_price,
            OrderType::Bracket {
                entry_limit: Some(limit),
                ..
            } => *limit,
            OrderType::Bracket {
                entry_limit: None, ..
            } => estimated_price,
            OrderType::Market => estimated_price,
            OrderType::StopLoss { stop_price } => *stop_price,
        };

        let position_value = effective_price.mul_qty(order.qty);

        // 4. Maximum Single Position Exposure Cap Check
        let max_position_allowed =
            BasisPoints(self.limits.max_position_bps).apply_to(account.cash_balance);
        if position_value > max_position_allowed {
            return Err(RiskRejection::MaxPositionExposureExceeded {
                position_value,
                max_allowed: max_position_allowed,
            });
        }

        // 5. Margin Sufficiency Check (for Buy orders)
        if order.side == OrderSide::Buy {
            let available = account.available_margin();
            if position_value > available {
                return Err(RiskRejection::InsufficientMargin {
                    required: position_value,
                    available,
                });
            }
        }

        // 6. 1% Account Risk Cap Check (on orders with Stop Loss)
        if let OrderType::Bracket { stop_loss, .. } = &order.order_type {
            let risk_per_share_paise = match order.side {
                OrderSide::Buy => {
                    if *stop_loss >= effective_price {
                        return Err(RiskRejection::InvalidStopLoss(
                            "Stop Loss must be strictly below Entry Price for Long orders"
                                .to_string(),
                        ));
                    }
                    effective_price.to_paise() - stop_loss.to_paise()
                }
                OrderSide::Sell => {
                    if *stop_loss <= effective_price {
                        return Err(RiskRejection::InvalidStopLoss(
                            "Stop Loss must be strictly above Entry Price for Short orders"
                                .to_string(),
                        ));
                    }
                    stop_loss.to_paise() - effective_price.to_paise()
                }
            };

            let total_risk_paise = risk_per_share_paise
                .checked_mul(order.qty as i64)
                .unwrap_or(i64::MAX);
            let total_risk = Money::from_paise(total_risk_paise);

            let max_allowed_risk =
                BasisPoints(self.limits.max_risk_bps).apply_to(account.cash_balance);
            if total_risk > max_allowed_risk {
                return Err(RiskRejection::RiskExceedsMaxAccountRisk {
                    risk_amount: total_risk,
                    max_allowed: max_allowed_risk,
                });
            }
        }

        Ok(())
    }

    /// Institutional position sizing calculator.
    /// Calculates the maximum allowed shares that strictly satisfy:
    /// 1. 1% Max Risk Cap (`max_allowed_risk / risk_per_share`)
    /// 2. Margin Limit (`available_margin / entry_price`)
    /// 3. Position Exposure Cap (`max_position_allowed / entry_price`)
    /// 4. Desk Fat-Finger Limit (`max_order_qty`)
    pub fn calculate_max_shares(
        &self,
        account: &TradingAccount,
        entry_price: Money,
        stop_loss: Money,
    ) -> u32 {
        if account.status != AccountStatus::Active
            || entry_price <= Money::zero()
            || stop_loss <= Money::zero()
            || entry_price == stop_loss
            || account.cash_balance <= Money::zero()
        {
            return 0;
        }

        let risk_per_share_paise = (entry_price.to_paise() - stop_loss.to_paise()).abs();
        if risk_per_share_paise == 0 {
            return 0;
        }

        // 1. Max shares by Risk Cap (e.g. 1%)
        let max_allowed_risk_paise =
            BasisPoints(self.limits.max_risk_bps).apply_to(account.cash_balance).to_paise();
        let max_by_risk = (max_allowed_risk_paise / risk_per_share_paise) as u32;

        // 2. Max shares by Margin
        let available_paise = account.available_margin().to_paise();
        let max_by_margin = if available_paise > 0 {
            (available_paise / entry_price.to_paise()) as u32
        } else {
            0
        };

        // 3. Max shares by Position Exposure Cap (e.g. 25%)
        let max_position_paise = BasisPoints(self.limits.max_position_bps)
            .apply_to(account.cash_balance)
            .to_paise();
        let max_by_position = (max_position_paise / entry_price.to_paise()) as u32;

        // Take the most conservative limit
        let safe_shares = max_by_risk
            .min(max_by_margin)
            .min(max_by_position)
            .min(self.limits.max_order_qty);

        safe_shares
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::account::{AccountId, UserId};
    use crate::models::order::OrderId;

    fn setup_test_account(balance_rupees: f64) -> TradingAccount {
        TradingAccount::new(
            AccountId(1001),
            UserId(42),
            Money::from_rupees(balance_rupees),
        )
    }

    fn make_test_order(order_type: OrderType, qty: u32) -> Order {
        Order::new(
            OrderId(1),
            UserId(42),
            AccountId(1001),
            "RELIANCE.NS",
            OrderSide::Buy,
            order_type,
            qty,
        )
        .unwrap()
    }

    #[test]
    fn test_valid_order_passes_risk_checks() {
        let account = setup_test_account(100_000.0);
        let risk_engine = PreTradeRiskEngine::new();

        // 10 shares @ Rs 2500 = Rs 25,000 (exactly 25% max position cap)
        // Stop loss @ Rs 2450 -> Risk per share Rs 50 -> Total risk Rs 500 (0.5% <= 1.0% max risk)
        let order = make_test_order(
            OrderType::Bracket {
                entry_limit: Some(Money::from_rupees(2500.0)),
                stop_loss: Money::from_rupees(2450.0),
                take_profit: Money::from_rupees(2600.0),
            },
            10,
        );

        let result = risk_engine.validate_order(&account, &order, Money::from_rupees(2500.0));
        assert!(result.is_ok());
    }

    #[test]
    fn test_risk_exceeds_1_percent_rejection() {
        let account = setup_test_account(100_000.0); // 1% risk = Rs 1,000
        let mut limits = RiskLimits::default();
        limits.max_position_bps = 5000; // Allow 50% position value to isolate the risk check
        let risk_engine = PreTradeRiskEngine::with_limits(limits);

        // Buy 15 shares @ Rs 2500. Stop @ Rs 2400 (Rs 100 risk per share).
        // Total risk = 15 * 100 = Rs 1,500 > Rs 1,000 (1% of 100,000)
        let order = make_test_order(
            OrderType::Bracket {
                entry_limit: Some(Money::from_rupees(2500.0)),
                stop_loss: Money::from_rupees(2400.0),
                take_profit: Money::from_rupees(2700.0),
            },
            15,
        );

        let result = risk_engine.validate_order(&account, &order, Money::from_rupees(2500.0));
        assert_eq!(
            result,
            Err(RiskRejection::RiskExceedsMaxAccountRisk {
                risk_amount: Money::from_rupees(1500.0),
                max_allowed: Money::from_rupees(1000.0),
            })
        );
    }

    #[test]
    fn test_insufficient_margin_rejection() {
        let mut account = setup_test_account(100_000.0);
        // Lock Rs 95,000 margin -> Only Rs 5,000 available
        account
            .allocate_margin(Money::from_rupees(95_000.0))
            .unwrap();

        let risk_engine = PreTradeRiskEngine::new();

        // Needs Rs 10,000 margin
        let order = make_test_order(
            OrderType::Limit {
                limit_price: Money::from_rupees(1000.0),
            },
            10,
        );

        let result = risk_engine.validate_order(&account, &order, Money::from_rupees(1000.0));
        assert_eq!(
            result,
            Err(RiskRejection::InsufficientMargin {
                required: Money::from_rupees(10_000.0),
                available: Money::from_rupees(5_000.0),
            })
        );
    }

    #[test]
    fn test_suspended_account_rejection() {
        let mut account = setup_test_account(100_000.0);
        account.status = AccountStatus::Suspended;

        let risk_engine = PreTradeRiskEngine::new();
        let order = make_test_order(OrderType::Market, 5);

        let result = risk_engine.validate_order(&account, &order, Money::from_rupees(2500.0));
        assert!(matches!(
            result,
            Err(RiskRejection::AccountNotActive { .. })
        ));
    }

    #[test]
    fn test_blocked_symbol_rejection() {
        let account = setup_test_account(100_000.0);
        let mut risk_engine = PreTradeRiskEngine::new();
        risk_engine.block_symbol("RELIANCE.NS");

        let order = make_test_order(OrderType::Market, 5);
        let result = risk_engine.validate_order(&account, &order, Money::from_rupees(2500.0));

        assert_eq!(
            result,
            Err(RiskRejection::SymbolBlocked {
                symbol: "RELIANCE.NS".to_string()
            })
        );
    }

    #[test]
    fn test_max_position_exposure_rejection() {
        let account = setup_test_account(100_000.0);
        let risk_engine = PreTradeRiskEngine::new(); // default 25% max position = Rs 25,000

        // Order value = 11 shares * Rs 2500 = Rs 27,500 > Rs 25,000
        let order = make_test_order(
            OrderType::Limit {
                limit_price: Money::from_rupees(2500.0),
            },
            11,
        );

        let result = risk_engine.validate_order(&account, &order, Money::from_rupees(2500.0));
        assert_eq!(
            result,
            Err(RiskRejection::MaxPositionExposureExceeded {
                position_value: Money::from_rupees(27_500.0),
                max_allowed: Money::from_rupees(25_000.0),
            })
        );
    }

    #[test]
    fn test_fat_finger_max_quantity_rejection() {
        let account = setup_test_account(10_000_000.0); // 1 Crore account
        let mut limits = RiskLimits::default();
        limits.max_order_qty = 500;
        let risk_engine = PreTradeRiskEngine::with_limits(limits);

        let order = make_test_order(OrderType::Market, 501);
        let result = risk_engine.validate_order(&account, &order, Money::from_rupees(100.0));

        assert_eq!(
            result,
            Err(RiskRejection::MaxOrderQuantityExceeded {
                requested_qty: 501,
                max_qty: 500,
            })
        );
    }

    #[test]
    fn test_position_sizer_calculator() {
        let account = setup_test_account(100_000.0);
        let risk_engine = PreTradeRiskEngine::new();
        // Account = Rs 100,000
        // 1% max risk = Rs 1,000
        // 25% max position = Rs 25,000
        // Entry: Rs 1,000, Stop Loss: Rs 980 -> Risk per share = Rs 20
        // Shares by Risk = 1,000 / 20 = 50 shares
        // Shares by Position Cap = 25,000 / 1,000 = 25 shares
        // Shares by Margin = 100,000 / 1,000 = 100 shares
        // Conservative output must be 25 shares!
        let safe_shares = risk_engine.calculate_max_shares(
            &account,
            Money::from_rupees(1000.0),
            Money::from_rupees(980.0),
        );
        assert_eq!(safe_shares, 25);
    }
}
