use super::error::{BeanError, Result};
use rust_decimal::Decimal;
use std::{fmt::Display, rc::Rc};

#[derive(Debug)]
pub enum BookingMethod {
	Strict,
	StrictWithSize,
	None,
	Average,
	FirstInFirstout,
	LastInFirstOut,
	HighestInFirstOut,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Commodity(Rc<str>);

impl Display for Commodity {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.0)
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct Account(Rc<str>);

#[derive(Debug)]
pub struct Amount {
	number: Decimal,
	commodity: Commodity,
}

impl Amount {
	pub fn new(number: Decimal, commodity: Commodity) -> Self {
		Self { number, commodity }
	}

	pub fn add(&self, rhs: &Amount) -> Result<Amount> {
		if self.commodity != rhs.commodity {
			Err(BeanError::CommodityMismatch(
				self.commodity.clone(),
				rhs.commodity.clone(),
			))
		} else {
			Ok(Amount::new(
				self.number + rhs.number,
				self.commodity.clone(),
			))
		}
	}

	pub fn sub(&self, rhs: &Amount) -> Result<Amount> {
		if self.commodity != rhs.commodity {
			Err(BeanError::CommodityMismatch(
				self.commodity.clone(),
				rhs.commodity.clone(),
			))
		} else {
			Ok(Amount::new(
				self.number - rhs.number,
				self.commodity.clone(),
			))
		}
	}

	pub fn mul(&self, number: Decimal) -> Amount {
		Amount::new(self.number * number, self.commodity.clone())
	}
}
