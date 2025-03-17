use super::error::{BeanError, Result};
use rust_decimal::prelude::*;

pub const ZERO: Decimal = Decimal::ZERO;
pub const HALF: Decimal = Decimal::from_parts(5, 0, 0, false, 1);
pub const ONE: Decimal = Decimal::ONE;
pub const TEN: Decimal = Decimal::TEN;

pub fn bean_d(s: &str) -> Result<Decimal> {
	// Try to parse the string normally first
	Decimal::from_str(s)
		.or_else(|_| {
			// Remove the commas and try again
			let s_no_commas = s.replace(",", "");
			Decimal::from_str(&s_no_commas)
		})
		.map_err(|e| BeanError::from(e))
}
