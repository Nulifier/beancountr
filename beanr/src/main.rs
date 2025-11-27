use std::rc::Rc;

use beancountr::parser::{parse_str, print_errors};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// does testing things
	Test,
}

fn main() {
	let cli = Cli::parse();

	match &cli.command {
		Commands::Test => {
			let filename: Rc<str> = Rc::from("filename.beancount");
			let src = r#"
			2025-01-01 txn "Cafe Mogador" "Lamb tagine with wine"
				Liabilities:CreditCard -37.45 USD
				Expenses:Restaurants
		"#;
			let (statements, errors) = parse_str(filename.clone(), src);

			print_errors(filename, src, errors);

			if let Some(statements) = statements {
				println!("{:#?}", statements);
			}
		}
	}
}
