//! Custom Financial Types Module for ApexDesk.

pub mod math;
pub mod money;

pub use math::{normalize_to_tick, BasisPoints, NSE_DEFAULT_TICK_SIZE_PAISE};
pub use money::Money;
