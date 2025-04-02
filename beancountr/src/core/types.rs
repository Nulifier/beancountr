use super::error::{BeanError, Result};
use rust_decimal::Decimal;
use std::{fmt::Display, rc::Rc, str::FromStr};

#[derive(Debug, Clone, PartialEq)]
pub enum BookingMethod {
	Strict,
	StrictWithSize,
	None,
	Average,
	FirstInFirstout,
	LastInFirstOut,
	HighestInFirstOut,
}

impl From<String> for BookingMethod {
	fn from(s: String) -> Self {
		match s.as_str() {
			"STRICT" => Self::Strict,
			"STRICT_WITH_SIZE" => Self::StrictWithSize,
			"NONE" => Self::None,
			"AVERAGE" => Self::Average,
			"FIFO" => Self::FirstInFirstout,
			"LIFO" => Self::LastInFirstOut,
			"HIFO" => Self::HighestInFirstOut,
			_ => panic!("Invalid booking method: {}", s),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct Commodity(Rc<str>);

impl FromStr for Commodity {
	type Err = BeanError;

	fn from_str(s: &str) -> Result<Self> {
		Ok(Self(Rc::from(s)))
	}
}

impl Display for Commodity {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.0)
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct Account(Rc<str>);

impl FromStr for Account {
	type Err = BeanError;

	fn from_str(s: &str) -> Result<Self> {
		Ok(Self(Rc::from(s)))
	}
}

impl From<Vec<String>> for Account {
	fn from(v: Vec<String>) -> Self {
		Self(Rc::from(v.join(":")))
	}
}

impl From<Vec<&str>> for Account {
	fn from(v: Vec<&str>) -> Self {
		Self(Rc::from(v.join(":")))
	}
}

#[derive(Debug, Clone, PartialEq)]
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
