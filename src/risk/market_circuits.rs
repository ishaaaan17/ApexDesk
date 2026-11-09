//! Indian Market Upper/Lower Circuit Guard for ApexDesk.
//! Enforces SEBI & NSE daily price band rules (2%, 5%, 10%, 20%, and custom bands).
//! Protects traders against:
//! 1. Placing orders outside the exchange-mandated daily price range.
//! 2. Upper Circuit (UC) trap: Buying into a frozen ceiling where zero sellers exist.
//! 3. Lower Circuit (LC) trap: Selling into a frozen floor where zero buyers exist.
//! 4. Supports dynamic band relaxation (as used by NSE for F&O stocks with cooling periods).

use std::collections::HashMap;

use crate::models::order::OrderSide;
use crate::types::math::{normalize_to_tick, BasisPoints, NSE_DEFAULT_TICK_SIZE_PAISE};
use crate::types::Money;

/// Standard SEBI/NSE circuit band tiers for Indian equities.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CircuitBand {
    /// 2% Band (200 BPS) - Highly speculative, SME, or GSM Stage-IV securities.
    TwoPercent,
    /// 5% Band (500 BPS) - Trade-to-Trade (T-Group) and ASM/GSM surveillance securities.
    FivePercent,
    /// 10% Band (1000 BPS) - Standard cash segment equities and initial F&O band.
    TenPercent,
    /// 20% Band (2000 BPS) - Standard large-cap non-F&O cash equities.
    TwentyPercent,
    /// Custom basis points (e.g. 1500 for relaxed 15% F&O band after cooling period).
    Custom(u32),
}

impl CircuitBand {
    /// Returns the circuit band size in Basis Points (1 BPS = 0.01%).
    #[inline]
    pub fn to_bps(&self) -> u32 {
        match self {
            CircuitBand::TwoPercent => 200,
            CircuitBand::FivePercent => 500,
            CircuitBand::TenPercent => 1000,
            CircuitBand::TwentyPercent => 2000,
            CircuitBand::Custom(bps) => *bps,
        }
    }

    /// Returns the percentage as a floating point number (e.g., 5.0 for 5%).
    #[inline]
    pub fn to_percentage(&self) -> f64 {
        self.to_bps() as f64 / 100.0
    }
}

/// The daily circuit limits for a specific traded security.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityCircuit {
    pub symbol: String,
    pub prev_close: Money,
    pub band: CircuitBand,
    pub lower_circuit: Money,
    pub upper_circuit: Money,
}

impl SecurityCircuit {
    /// Computes Upper and Lower circuits from previous day's close price,
    /// strictly snapping values to NSE's 5-paise tick size grid.
    pub fn new(symbol: impl Into<String>, prev_close: Money, band: CircuitBand) -> Self {
        let bps = band.to_bps();
        let band_offset = BasisPoints(bps).apply_to(prev_close);

        let raw_uc = prev_close + band_offset;
        let raw_lc = if prev_close > band_offset {
            prev_close - band_offset
        } else {
            Money::from_paise(NSE_DEFAULT_TICK_SIZE_PAISE)
        };

        let upper_circuit = normalize_to_tick(raw_uc, NSE_DEFAULT_TICK_SIZE_PAISE);
        let lower_circuit = normalize_to_tick(raw_lc, NSE_DEFAULT_TICK_SIZE_PAISE);

        SecurityCircuit {
            symbol: symbol.into(),
            prev_close,
            band,
            lower_circuit,
            upper_circuit,
        }
    }

    /// Checks if a price is strictly within the permitted circuit limits `[LC, UC]`.
    #[inline]
    pub fn is_within_band(&self, price: Money) -> bool {
        price >= self.lower_circuit && price <= self.upper_circuit
    }
}

/// Violations produced when an order violates circuit boundaries or lockout rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitViolation {
    PriceAboveUpperCircuit {
        price: Money,
        upper_circuit: Money,
        symbol: String,
    },
    PriceBelowLowerCircuit {
        price: Money,
        lower_circuit: Money,
        symbol: String,
    },
    UpperCircuitLockout {
        price: Money,
        upper_circuit: Money,
        symbol: String,
    },
    LowerCircuitLockout {
        price: Money,
        lower_circuit: Money,
        symbol: String,
    },
    SecurityNotFound {
        symbol: String,
    },
}

/// Registry and evaluator for Indian market upper and lower circuit guards.
#[derive(Debug, Clone)]
pub struct MarketCircuitGuard {
    circuits: HashMap<String, SecurityCircuit>,
    block_circuit_lockouts: bool,
}

impl Default for MarketCircuitGuard {
    fn default() -> Self {
        Self {
            circuits: HashMap::new(),
            block_circuit_lockouts: true,
        }
    }
}

impl MarketCircuitGuard {
    /// Creates a new circuit guard with circuit freeze/lockout protections enabled by default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggles whether orders hitting exact circuit limits are rejected (UC buy lockout / LC sell lockout).
    pub fn set_block_lockouts(&mut self, block: bool) {
        self.block_circuit_lockouts = block;
    }

    /// Registers or updates the circuit limits for a security using previous day's close.
    pub fn register_security(
        &mut self,
        symbol: impl Into<String>,
        prev_close: Money,
        band: CircuitBand,
    ) -> &SecurityCircuit {
        let sym = symbol.into();
        let circuit = SecurityCircuit::new(sym.clone(), prev_close, band);
        self.circuits.insert(sym.clone(), circuit);
        self.circuits.get(&sym).unwrap()
    }

    /// Retrieves the circuit limits for a given security.
    pub fn get_circuit(&self, symbol: &str) -> Option<&SecurityCircuit> {
        self.circuits.get(symbol)
    }

    /// Relaxes the circuit band for a security (e.g. F&O band expansion from 10% to 15% after cooling).
    pub fn relax_band(
        &mut self,
        symbol: &str,
        new_band: CircuitBand,
    ) -> Result<&SecurityCircuit, CircuitViolation> {
        let prev_close = match self.circuits.get(symbol) {
            Some(c) => c.prev_close,
            None => {
                return Err(CircuitViolation::SecurityNotFound {
                    symbol: symbol.to_string(),
                })
            }
        };

        let new_circuit = SecurityCircuit::new(symbol, prev_close, new_band);
        self.circuits.insert(symbol.to_string(), new_circuit);
        Ok(self.circuits.get(symbol).unwrap())
    }

    /// Validates an intended order price against the security's circuit boundaries.
    ///
    /// # Rules Enforced:
    /// 1. `price > upper_circuit`: Rejected (`PriceAboveUpperCircuit`).
    /// 2. `price < lower_circuit`: Rejected (`PriceBelowLowerCircuit`).
    /// 3. `side == Buy && price == upper_circuit`: If lockouts enabled, rejected (`UpperCircuitLockout`).
    /// 4. `side == Sell && price == lower_circuit`: If lockouts enabled, rejected (`LowerCircuitLockout`).
    pub fn validate_order_price(
        &self,
        symbol: &str,
        side: OrderSide,
        price: Money,
    ) -> Result<(), CircuitViolation> {
        let circuit = match self.circuits.get(symbol) {
            Some(c) => c,
            None => {
                return Err(CircuitViolation::SecurityNotFound {
                    symbol: symbol.to_string(),
                })
            }
        };

        // 1. Strict Upper Circuit ceiling check
        if price > circuit.upper_circuit {
            return Err(CircuitViolation::PriceAboveUpperCircuit {
                price,
                upper_circuit: circuit.upper_circuit,
                symbol: symbol.to_string(),
            });
        }

        // 2. Strict Lower Circuit floor check
        if price < circuit.lower_circuit {
            return Err(CircuitViolation::PriceBelowLowerCircuit {
                price,
                lower_circuit: circuit.lower_circuit,
                symbol: symbol.to_string(),
            });
        }

        // 3. Circuit Lockout Protection (Prevents getting trapped with zero liquidity)
        if self.block_circuit_lockouts {
            if side == OrderSide::Buy && price == circuit.upper_circuit {
                return Err(CircuitViolation::UpperCircuitLockout {
                    price,
                    upper_circuit: circuit.upper_circuit,
                    symbol: symbol.to_string(),
                });
            }

            if side == OrderSide::Sell && price == circuit.lower_circuit {
                return Err(CircuitViolation::LowerCircuitLockout {
                    price,
                    lower_circuit: circuit.lower_circuit,
                    symbol: symbol.to_string(),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_calculation_20_percent_large_cap() {
        // Reliance previous close = Rs 2,500.00
        // 20% band = Rs 500.00 -> UC = Rs 3,000.00, LC = Rs 2,000.00
        let circuit = SecurityCircuit::new(
            "RELIANCE.NS",
            Money::from_rupees(2500.0),
            CircuitBand::TwentyPercent,
        );

        assert_eq!(circuit.upper_circuit, Money::from_rupees(3000.0));
        assert_eq!(circuit.lower_circuit, Money::from_rupees(2000.0));
        assert!(circuit.is_within_band(Money::from_rupees(2500.0)));
        assert!(circuit.is_within_band(Money::from_rupees(2000.0)));
        assert!(circuit.is_within_band(Money::from_rupees(3000.0)));
        assert!(!circuit.is_within_band(Money::from_rupees(3005.0)));
        assert!(!circuit.is_within_band(Money::from_rupees(1995.0)));
    }

    #[test]
    fn test_circuit_calculation_5_percent_small_cap() {
        // Suzlon previous close = Rs 100.00
        // 5% band = Rs 5.00 -> UC = Rs 105.00, LC = Rs 95.00
        let circuit = SecurityCircuit::new(
            "SUZLON.NS",
            Money::from_rupees(100.0),
            CircuitBand::FivePercent,
        );

        assert_eq!(circuit.upper_circuit, Money::from_rupees(105.0));
        assert_eq!(circuit.lower_circuit, Money::from_rupees(95.0));
    }

    #[test]
    fn test_price_above_upper_circuit_rejected() {
        let mut guard = MarketCircuitGuard::new();
        guard.register_security(
            "TCS.NS",
            Money::from_rupees(3500.0),
            CircuitBand::TenPercent, // UC = 3850, LC = 3150
        );

        // Buy order at Rs 3855 (above UC) -> Rejected
        let result = guard.validate_order_price(
            "TCS.NS",
            OrderSide::Buy,
            Money::from_rupees(3855.0),
        );

        assert_eq!(
            result,
            Err(CircuitViolation::PriceAboveUpperCircuit {
                price: Money::from_rupees(3855.0),
                upper_circuit: Money::from_rupees(3850.0),
                symbol: "TCS.NS".to_string(),
            })
        );
    }

    #[test]
    fn test_price_below_lower_circuit_rejected() {
        let mut guard = MarketCircuitGuard::new();
        guard.register_security(
            "TCS.NS",
            Money::from_rupees(3500.0),
            CircuitBand::TenPercent, // UC = 3850, LC = 3150
        );

        // Sell order at Rs 3140 (below LC) -> Rejected
        let result = guard.validate_order_price(
            "TCS.NS",
            OrderSide::Sell,
            Money::from_rupees(3140.0),
        );

        assert_eq!(
            result,
            Err(CircuitViolation::PriceBelowLowerCircuit {
                price: Money::from_rupees(3140.0),
                lower_circuit: Money::from_rupees(3150.0),
                symbol: "TCS.NS".to_string(),
            })
        );
    }

    #[test]
    fn test_upper_circuit_lockout_blocks_buy_orders() {
        let mut guard = MarketCircuitGuard::new();
        guard.register_security(
            "TCS.NS",
            Money::from_rupees(3500.0),
            CircuitBand::TenPercent, // UC = 3850.00
        );

        // Buying exactly at Upper Circuit (where no sellers exist) is blocked
        let buy_res = guard.validate_order_price(
            "TCS.NS",
            OrderSide::Buy,
            Money::from_rupees(3850.0),
        );
        assert_eq!(
            buy_res,
            Err(CircuitViolation::UpperCircuitLockout {
                price: Money::from_rupees(3850.0),
                upper_circuit: Money::from_rupees(3850.0),
                symbol: "TCS.NS".to_string(),
            })
        );

        // Selling at Upper Circuit is permitted (providing exit liquidity)
        let sell_res = guard.validate_order_price(
            "TCS.NS",
            OrderSide::Sell,
            Money::from_rupees(3850.0),
        );
        assert!(sell_res.is_ok());
    }

    #[test]
    fn test_lower_circuit_lockout_blocks_sell_orders() {
        let mut guard = MarketCircuitGuard::new();
        guard.register_security(
            "TCS.NS",
            Money::from_rupees(3500.0),
            CircuitBand::TenPercent, // LC = 3150.00
        );

        // Selling exactly at Lower Circuit (where no buyers exist) is blocked
        let sell_res = guard.validate_order_price(
            "TCS.NS",
            OrderSide::Sell,
            Money::from_rupees(3150.0),
        );
        assert_eq!(
            sell_res,
            Err(CircuitViolation::LowerCircuitLockout {
                price: Money::from_rupees(3150.0),
                lower_circuit: Money::from_rupees(3150.0),
                symbol: "TCS.NS".to_string(),
            })
        );

        // Buying at Lower Circuit is permitted (providing bid liquidity)
        let buy_res = guard.validate_order_price(
            "TCS.NS",
            OrderSide::Buy,
            Money::from_rupees(3150.0),
        );
        assert!(buy_res.is_ok());
    }

    #[test]
    fn test_dynamic_band_relaxation_for_fo_stocks() {
        let mut guard = MarketCircuitGuard::new();
        guard.register_security(
            "INFY.NS",
            Money::from_rupees(1500.0),
            CircuitBand::TenPercent, // UC = 1650.00
        );

        let initial_uc = guard.get_circuit("INFY.NS").unwrap().upper_circuit;
        assert_eq!(initial_uc, Money::from_rupees(1650.0));

        // After cooling period, exchange relaxes band to 15% (1500 BPS)
        let updated = guard
            .relax_band("INFY.NS", CircuitBand::Custom(1500))
            .unwrap();
        // 15% on 1500 = 225 -> UC becomes 1725.00
        assert_eq!(updated.upper_circuit, Money::from_rupees(1725.0));
        assert_eq!(updated.lower_circuit, Money::from_rupees(1275.0));
    }

    #[test]
    fn test_unknown_security_returns_error() {
        let guard = MarketCircuitGuard::new();
        let res = guard.validate_order_price("UNKNOWN.NS", OrderSide::Buy, Money::from_rupees(100.0));
        assert_eq!(
            res,
            Err(CircuitViolation::SecurityNotFound {
                symbol: "UNKNOWN.NS".to_string()
            })
        );
    }
}
