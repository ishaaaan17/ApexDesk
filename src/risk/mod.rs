//! Pre-trade risk controls, circuit breakers, and capital protection module.

pub mod pre_trade;

pub use pre_trade::{PreTradeRiskEngine, RiskLimits, RiskRejection};
