//! Double-Entry Accounting Ledger Module.
//! Guarantees that financial records across 10,000+ users maintain mathematical
//! integrity through the immutable law of accounting: sum(Debits) == sum(Credits).

use crate::models::account::UserId;
use crate::types::Money;

/// Standard categories in a double-entry Chart of Accounts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LedgerAccountType {
    /// Liquid user funds available for trading.
    AssetCash,
    /// Capital locked in open equity positions.
    AssetPosition(String),
    /// Platform liability representing trader virtual equity.
    LiabilityUserEquity,
    /// Regulatory fees, brokerage, STT, and exchange turnover taxes paid.
    ExpenseTradingFees,
    /// Realized gross trading loss.
    ExpenseTradingLoss,
    /// Realized gross market profit.
    RevenueTradingGains,
}

/// A single leg (debit or credit) within a double-entry financial transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntry {
    pub account: LedgerAccountType,
    pub debit: Money,
    pub credit: Money,
}

impl LedgerEntry {
    /// Creates a debit entry (increases Assets/Expenses, decreases Liabilities/Revenue).
    pub fn debit(account: LedgerAccountType, amount: Money) -> Self {
        LedgerEntry {
            account,
            debit: amount,
            credit: Money::zero(),
        }
    }

    /// Creates a credit entry (increases Liabilities/Revenue, decreases Assets/Expenses).
    pub fn credit(account: LedgerAccountType, amount: Money) -> Self {
        LedgerEntry {
            account,
            debit: Money::zero(),
            credit: amount,
        }
    }
}

/// Errors returned by the double-entry accounting engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    UnbalancedTransaction { total_debits: Money, total_credits: Money },
    EmptyTransaction,
    NegativeEntryAmount,
}

/// An immutable financial transaction containing balanced debits and credits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub tx_id: u64,
    pub user_id: UserId,
    pub description: String,
    pub entries: Vec<LedgerEntry>,
}

impl Transaction {
    /// Validates and constructs a transaction. Rejects any transaction where sum(debits) != sum(credits).
    pub fn new(
        tx_id: u64,
        user_id: UserId,
        description: impl Into<String>,
        entries: Vec<LedgerEntry>,
    ) -> Result<Self, LedgerError> {
        if entries.is_empty() {
            return Err(LedgerError::EmptyTransaction);
        }

        let mut total_debits = Money::zero();
        let mut total_credits = Money::zero();

        for entry in &entries {
            if entry.debit.is_negative() || entry.credit.is_negative() {
                return Err(LedgerError::NegativeEntryAmount);
            }
            total_debits += entry.debit;
            total_credits += entry.credit;
        }

        if total_debits != total_credits {
            return Err(LedgerError::UnbalancedTransaction {
                total_debits,
                total_credits,
            });
        }

        if total_debits.is_zero() {
            return Err(LedgerError::EmptyTransaction);
        }

        Ok(Transaction {
            tx_id,
            user_id,
            description: description.into(),
            entries,
        })
    }
}

/// The multi-tenant immutable double-entry ledger.
#[derive(Debug, Default, Clone)]
pub struct Ledger {
    transactions: Vec<Transaction>,
    next_tx_id: u64,
}

impl Ledger {
    /// Creates a fresh ledger instance.
    pub fn new() -> Self {
        Ledger {
            transactions: Vec::new(),
            next_tx_id: 1,
        }
    }

    /// Records initial user capital deposit into the double-entry books.
    pub fn record_deposit(&mut self, user_id: UserId, amount: Money) -> Result<&Transaction, LedgerError> {
        let entries = vec![
            LedgerEntry::debit(LedgerAccountType::AssetCash, amount),
            LedgerEntry::credit(LedgerAccountType::LiabilityUserEquity, amount),
        ];
        let tx = Transaction::new(self.next_tx_id, user_id, "User Initial Capital Deposit", entries)?;
        self.next_tx_id += 1;
        self.transactions.push(tx);
        Ok(self.transactions.last().unwrap())
    }

    /// Records capital allocation into a stock position (e.g. buying RELIANCE).
    pub fn record_position_entry(
        &mut self,
        user_id: UserId,
        symbol: &str,
        cost: Money,
    ) -> Result<&Transaction, LedgerError> {
        let entries = vec![
            LedgerEntry::debit(LedgerAccountType::AssetPosition(symbol.to_string()), cost),
            LedgerEntry::credit(LedgerAccountType::AssetCash, cost),
        ];
        let tx = Transaction::new(
            self.next_tx_id,
            user_id,
            format!("Open Long Position in {}", symbol),
            entries,
        )?;
        self.next_tx_id += 1;
        self.transactions.push(tx);
        Ok(self.transactions.last().unwrap())
    }

    /// Records a settled trade with gross gain/loss, position closure, and regulatory fees.
    pub fn record_trade_settlement(
        &mut self,
        user_id: UserId,
        symbol: &str,
        cost_basis: Money,
        net_cash_returned: Money,
        fees: Money,
        gross_pnl: Money,
    ) -> Result<&Transaction, LedgerError> {
        let mut entries = vec![
            LedgerEntry::debit(LedgerAccountType::AssetCash, net_cash_returned),
            LedgerEntry::credit(LedgerAccountType::AssetPosition(symbol.to_string()), cost_basis),
        ];

        if fees.is_positive() {
            entries.push(LedgerEntry::debit(LedgerAccountType::ExpenseTradingFees, fees));
        }

        if gross_pnl.is_positive() {
            entries.push(LedgerEntry::credit(LedgerAccountType::RevenueTradingGains, gross_pnl));
        } else if gross_pnl.is_negative() {
            entries.push(LedgerEntry::debit(LedgerAccountType::ExpenseTradingLoss, gross_pnl.abs()));
        }

        let tx = Transaction::new(
            self.next_tx_id,
            user_id,
            format!("Settle Position in {}", symbol),
            entries,
        )?;
        self.next_tx_id += 1;
        self.transactions.push(tx);
        Ok(self.transactions.last().unwrap())
    }

    /// Verifies the mathematical integrity of the entire ledger.
    /// In an audit, total debits MUST equal total credits across all 10,000+ users!
    pub fn audit_integrity(&self) -> bool {
        let mut sum_debits = Money::zero();
        let mut sum_credits = Money::zero();

        for tx in &self.transactions {
            for entry in &tx.entries {
                sum_debits += entry.debit;
                sum_credits += entry.credit;
            }
        }
        sum_debits == sum_credits
    }

    /// Returns the total number of immutable transactions recorded.
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_deposit_transaction() {
        let mut ledger = Ledger::new();
        let deposit = Money::from_rupees(100000.0);
        let tx = ledger.record_deposit(UserId(1), deposit).unwrap();

        assert_eq!(tx.entries.len(), 2);
        assert_eq!(tx.entries[0].debit, deposit);
        assert_eq!(tx.entries[1].credit, deposit);
        assert!(ledger.audit_integrity());
    }

    #[test]
    fn test_unbalanced_transaction_rejected() {
        let entries = vec![
            LedgerEntry::debit(LedgerAccountType::AssetCash, Money::from_rupees(100.0)),
            LedgerEntry::credit(LedgerAccountType::LiabilityUserEquity, Money::from_rupees(90.0)),
        ];
        let res = Transaction::new(1, UserId(1), "Unbalanced Test", entries);
        assert_eq!(
            res,
            Err(LedgerError::UnbalancedTransaction {
                total_debits: Money::from_rupees(100.0),
                total_credits: Money::from_rupees(90.0),
            })
        );
    }

    #[test]
    fn test_complete_trade_lifecycle_double_entry() {
        let mut ledger = Ledger::new();
        let user = UserId(42);

        // 1. User deposits Rs 1 Lakh
        ledger.record_deposit(user, Money::from_rupees(100000.0)).unwrap();

        // 2. Buy Rs 25,000 worth of RELIANCE
        let cost = Money::from_rupees(25000.0);
        ledger.record_position_entry(user, "RELIANCE.NS", cost).unwrap();

        // 3. Settle trade with Rs 2,000 gross gain, Rs 138.64 taxes, Rs 1,861.36 net gain
        let net_cash = Money::from_paise(2686136);
        let fees = Money::from_paise(13864);
        let gross_gain = Money::from_paise(200000);

        ledger.record_trade_settlement(user, "RELIANCE.NS", cost, net_cash, fees, gross_gain).unwrap();

        // Audit check across all transactions
        assert_eq!(ledger.transaction_count(), 3);
        assert!(ledger.audit_integrity());
    }

    #[test]
    fn test_empty_transaction_rejected() {
        let res = Transaction::new(1, UserId(1), "Empty", vec![]);
        assert_eq!(res, Err(LedgerError::EmptyTransaction));
    }
}
