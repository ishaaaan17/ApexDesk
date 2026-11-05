//! Broker execution, matching engine, and simulated fills module.

pub mod matching_engine;

pub use matching_engine::{ExecutionFill, MatchResult, MatchingEngine};
