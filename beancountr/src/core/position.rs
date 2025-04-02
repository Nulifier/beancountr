use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::rc::Rc;

use super::types::Commodity;

#[derive(Debug, Clone, PartialEq)]
pub struct Cost {
	/// Per-unit cost.
	number: Decimal,

	// Cost Commodity
	commodity: Commodity,

	/// Date the lot was created at.
	/// There should always be a valid date.
	date: NaiveDate,

	/// A string for the label of this lot, if provided.
	label: Option<Rc<str>>,
}

/// A stand-in for an "incomplete" Cost, that is, a container all the data that
/// was provided by the user in the input in order to resolve this lot to a
/// particular lot and produce an instance of Cost. Any of the fields of this
/// object may be left unspecified, in which case they take the special value
/// "NA" (see below), if the field was absent from the input.
#[derive(Debug, Clone, PartialEq)]
pub struct CostSpec {
	number_per: Option<Decimal>,
	number_total: Option<Decimal>,
	commodity: Option<Commodity>,
	date: Option<NaiveDate>,
	label: Option<Rc<str>>,
	merge: Option<bool>,
}

// Either a cost or a cost spec.
#[derive(Debug, Clone, PartialEq)]
pub enum CostOrSpec {
	Cost(Cost),
	Spec(CostSpec),
}
