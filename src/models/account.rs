//! Multi-Tenant User & Account State Model.
//! Provides thread-safe, isolated state tracking for 10,000+ concurrent users
//! including available cash, locked margin, realized P&L, and account lifecycles.

use crate::types::Money;

/// Unique identifier for a platform user.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(pub u64);

/// Unique identifier for a trading account.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountId(pub u64);

/// Operational status of a trading account.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AccountStatus {
    Active,
    Suspended,
    Liquidating,
    Closed,
}

/// Errors returned by account operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountError {
    InsufficientFunds { available: Money, required: Money },
    AccountNotActive { status: AccountStatus },
    InvalidAmount,
    MarginOverflow,
}

/// Core multi-tenant trading account state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradingAccount {
    pub account_id: AccountId,
    pub user_id: UserId,
    pub status: AccountStatus,
    pub initial_deposit: Money,
    pub cash_balance: Money,
    pub allocated_margin: Money,
    pub realized_pnl: Money,
    pub total_fees_paid: Money,
}

impl TradingAccount {
    /// Creates a new active trading account with an initial cash deposit.
    pub fn new(account_id: AccountId, user_id: UserId, initial_deposit: Money) -> Self {
        TradingAccount {
            account_id,
            user_id,
            status: AccountStatus::Active,
            initial_deposit,
            cash_balance: initial_deposit,
            allocated_margin: Money::zero(),
            realized_pnl: Money::zero(),
            total_fees_paid: Money::zero(),
        }
    }

    /// Available unencumbered cash balance that can be used to fund new orders.
    #[inline]
    pub fn available_margin(&self) -> Money {
        if self.cash_balance > self.allocated_margin {
            self.cash_balance - self.allocated_margin
        } else {
            Money::zero()
        }
    }

    /// Checks if the account has sufficient available margin to fund an order.
    #[inline]
    pub fn can_allocate(&self, amount: Money) -> bool {
        self.status == AccountStatus::Active && self.available_margin() >= amount
    }

    /// Locks required margin for an open position or active order.
    pub fn allocate_margin(&mut self, amount: Money) -> Result<(), AccountError> {
        if self.status != AccountStatus::Active {
            return Err(AccountError::AccountNotActive { status: self.status });
        }
        if !amount.is_positive() {
            return Err(AccountError::InvalidAmount);
        }
        let available = self.available_margin();
        if available < amount {
            return Err(AccountError::InsufficientFunds {
                available,
                required: amount,
            });
        }
        self.allocated_margin += amount;
        Ok(())
    }

    /// Releases previously locked margin back to available balance when a position closes.
    pub fn release_margin(&mut self, amount: Money) -> Result<(), AccountError> {
        if !amount.is_positive() {
            return Err(AccountError::InvalidAmount);
        }
        if amount > self.allocated_margin {
            return Err(AccountError::MarginOverflow);
        }
        self.allocated_margin -= amount;
        Ok(())
    }

    /// Credits or debits net realized P&L and fees after a trade settlement.
    pub fn settle_trade(&mut self, net_pnl: Money, fees: Money) {
        self.cash_balance += net_pnl;
        self.realized_pnl += net_pnl;
        self.total_fees_paid += fees;
    }

    /// Deposits additional cash into the account.
    pub fn deposit(&mut self, amount: Money) -> Result<(), AccountError> {
        if self.status == AccountStatus::Closed {
            return Err(AccountError::AccountNotActive { status: self.status });
        }
        if !amount.is_positive() {
            return Err(AccountError::InvalidAmount);
        }
        self.cash_balance += amount;
        Ok(())
    }

    /// Withdraws free cash from the account.
    pub fn withdraw(&mut self, amount: Money) -> Result<(), AccountError> {
        if self.status != AccountStatus::Active {
            return Err(AccountError::AccountNotActive { status: self.status });
        }
        if !amount.is_positive() {
            return Err(AccountError::InvalidAmount);
        }
        let available = self.available_margin();
        if available < amount {
            return Err(AccountError::InsufficientFunds {
                available,
                required: amount,
            });
        }
        self.cash_balance -= amount;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_account() -> TradingAccount {
        TradingAccount::new(
            AccountId(101),
            UserId(1),
            Money::from_rupees(100000.0), // Rs 1 Lakh starting balance
        )
    }

    #[test]
    fn test_initial_account_state() {
        let acc = setup_test_account();
        assert_eq!(acc.status, AccountStatus::Active);
        assert_eq!(acc.cash_balance, Money::from_rupees(100000.0));
        assert_eq!(acc.available_margin(), Money::from_rupees(100000.0));
        assert_eq!(acc.allocated_margin, Money::zero());
        assert_eq!(acc.realized_pnl, Money::zero());
    }

    #[test]
    fn test_margin_allocation_and_release() {
        let mut acc = setup_test_account();
        let margin_req = Money::from_rupees(25000.0);

        // Lock Rs 25,000 for a trade
        assert!(acc.can_allocate(margin_req));
        acc.allocate_margin(margin_req).unwrap();

        assert_eq!(acc.allocated_margin, margin_req);
        assert_eq!(acc.available_margin(), Money::from_rupees(75000.0));
        assert_eq!(acc.cash_balance, Money::from_rupees(100000.0)); // Total cash still 1 Lakh

        // Release margin upon trade exit
        acc.release_margin(margin_req).unwrap();
        assert_eq!(acc.allocated_margin, Money::zero());
        assert_eq!(acc.available_margin(), Money::from_rupees(100000.0));
    }

    #[test]
    fn test_insufficient_margin_rejection() {
        let mut acc = setup_test_account();
        let excess_req = Money::from_rupees(105000.0); // More than Rs 1 Lakh available

        assert!(!acc.can_allocate(excess_req));
        let res = acc.allocate_margin(excess_req);
        assert_eq!(
            res,
            Err(AccountError::InsufficientFunds {
                available: Money::from_rupees(100000.0),
                required: excess_req,
            })
        );
    }

    #[test]
    fn test_settle_winning_trade() {
        let mut acc = setup_test_account();
        let profit = Money::from_rupees(2500.0);
        let fees = Money::from_rupees(120.0);

        acc.settle_trade(profit, fees);

        assert_eq!(acc.cash_balance, Money::from_rupees(102500.0));
        assert_eq!(acc.realized_pnl, profit);
        assert_eq!(acc.total_fees_paid, fees);
    }

    #[test]
    fn test_settle_losing_trade() {
        let mut acc = setup_test_account();
        let loss = Money::from_rupees(-1200.0);
        let fees = Money::from_rupees(90.0);

        acc.settle_trade(loss, fees);

        assert_eq!(acc.cash_balance, Money::from_rupees(98800.0));
        assert_eq!(acc.realized_pnl, loss);
        assert_eq!(acc.total_fees_paid, fees);
    }

    #[test]
    fn test_withdrawal_cannot_exceed_available_margin() {
        let mut acc = setup_test_account();
        acc.allocate_margin(Money::from_rupees(60000.0)).unwrap();

        // Available is Rs 40,000. Trying to withdraw Rs 50,000 must fail!
        let res = acc.withdraw(Money::from_rupees(50000.0));
        assert!(res.is_err());

        // Withdrawing Rs 30,000 must succeed
        assert!(acc.withdraw(Money::from_rupees(30000.0)).is_ok());
        assert_eq!(acc.cash_balance, Money::from_rupees(70000.0));
    }

    #[test]
    fn test_suspended_account_blocks_trades() {
        let mut acc = setup_test_account();
        acc.status = AccountStatus::Suspended;

        assert!(!acc.can_allocate(Money::from_rupees(1000.0)));
        let res = acc.allocate_margin(Money::from_rupees(1000.0));
        assert_eq!(
            res,
            Err(AccountError::AccountNotActive {
                status: AccountStatus::Suspended
            })
        );
    }
}
