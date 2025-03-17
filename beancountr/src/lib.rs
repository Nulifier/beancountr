use rust_decimal::prelude::*;

pub mod core;
pub mod loader;
pub mod parser; // TODO: Change back to private

pub fn test() {
	let a = Decimal::from_str("9000.00").unwrap();
	let b = Decimal::from_str("0.93324").unwrap();

	println!("Test {} ", a / b);
}
