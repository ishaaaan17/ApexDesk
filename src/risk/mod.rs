//! Pre-trade risk controls, circuit breakers, and capital protection module.

pub mod circuit_breaker;
pub mod pre_trade;

pub use circuit_breaker::{
    CircuitBreakerConfig, CircuitState, DailyCircuitBreaker, PanicSummary, TripReason,
};
pub use pre_trade::{PreTradeRiskEngine, RiskLimits, RiskRejection};
