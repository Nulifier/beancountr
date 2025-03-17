use std::fmt;

use super::types::Commodity;

#[derive(Debug)]
pub enum BeanError {
	DecimalError(rust_decimal::Error),
	CommodityMismatch(Commodity, Commodity),
}

impl std::error::Error for BeanError {}

impl fmt::Display for BeanError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::DecimalError(e) => e.fmt(f),
			Self::CommodityMismatch(lhs, rhs) => write!(
				f,
				"Unmatching currencies for operation on {} and {}",
				lhs, rhs
			),
		}
	}
}

impl From<rust_decimal::Error> for BeanError {
	fn from(value: rust_decimal::Error) -> Self {
		BeanError::DecimalError(value)
	}
}

pub type Result<T> = std::result::Result<T, BeanError>;
