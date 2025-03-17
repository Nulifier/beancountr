use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::{
	position::CostOrSpec,
	types::{Account, Amount, BookingMethod, Commodity},
};

#[derive(Debug)]
pub enum Metadata {
	String(String),
	Account(Account),
	Commodity(Commodity),
	Date(NaiveDate),
	Tags(HashSet<String>),
	Number(Decimal),
	Amount(Amount),
}

pub type MetadataMap = HashMap<String, Metadata>;

#[derive(Debug)]
pub struct Posting {
	account: Account,
	units: Option<Amount>,
	cost: Option<CostOrSpec>,
	price: Option<Amount>,
	flag: Option<char>,
	meta: MetadataMap,
}

#[derive(Debug)]
pub struct Directive {
	date: NaiveDate,
	kind: DirectiveKind,
	meta: MetadataMap,
}

#[derive(Debug)]
pub enum DirectiveKind {
	Open(Account, Vec<Commodity>, Option<BookingMethod>),
	Close(Account),
	Commodity(Commodity),
	Pad {
		account: Account,
		source_account: Account,
	},
	Balance {
		account: Account,
		amount: Amount,
		tolerance: Option<Decimal>,
		diff_amount: Option<Amount>,
	},
	Transaction {
		flag: Option<char>,
		payee: Option<String>,
		narration: Option<String>,
		tags: HashSet<String>,
		links: HashSet<String>,
		postings: Vec<Posting>,
	},
	Note {
		account: Account,
		comment: String,
		tags: HashSet<String>,
		links: HashSet<String>,
	},
	Event {
		kind: String,
		description: String,
	},
	Query {
		name: String,
		query: String,
	},
	Price {
		commodity: Commodity,
		amount: Amount,
	},
	Document {
		account: Account,
		filename: String,
		tags: HashSet<String>,
		links: HashSet<String>,
	},
	/// A custom directive. This directive can be used to implement new
	/// experimental dated features in the Beancount file. This is meant as an
	/// intermediate measure to be used when you would need to implement a new
	/// directive in a plugin. These directives will be parsed liberally... any
	/// list of tokens are supported. All that is required is some unique name
	/// for them that acts as a "type". These directives are included in the
	/// stream and a plugin should be able to gather them.
	Custom {
		/// A string that represents the type of the directive.
		kind: String,
		/// A list of values of various simple types supported by the grammar.
		/// (Note that this list is not enforced to be consistent for all
		/// directives of the same type by the parser.)
		values: Vec<Metadata>,
	},
}
