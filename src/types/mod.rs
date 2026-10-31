//! Custom Financial Types Module for ApexDesk.

pub mod math;
pub mod money;

pub use math::{
    calculate_bps_between, calculate_return_pct, normalize_to_tick, BasisPoints,
    NSE_DEFAULT_TICK_SIZE_PAISE,
};
pub use money::Money;
