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
			option "title" "My Beancount File"
			plugin "beancount.plugins.example"
			plugin "beancount.plugins.example" "arg1"
			include "file.beancount"
			2025-01-01 open Assets:US:B-of-A:Checking USD, CAD "NONE"
				value: 123.456
			2025-01-01 close Assets:US:B-of-A:Checking
			2025-01-01 commodity AAPL


			2025-01-01 pad Assets:US Equity:Opening-Balances
			2025-01-01 balance Assets:US 319.020 RGAGX
			2025-01-01 balance Assets:US 319.020 ~ 0.002 RGAGX
			2025-01-01 note Assets:US "This is a note"
			2025-01-01 note Assets:US "This is a note" #test-tag ^test-link
			2025-01-01 event "location" "Paris, France"
			2025-01-01 query "france-balances" "
				SELECT account, sum(position) WHERE ‘trip-france-2014’ in tags"
			2025-01-01 price HOOL 579.18 USD
			2025-01-01 document Liabilities:CreditCard "/home/joe/stmts/apr-2014.pdf" #test-tag ^test-link
			2025-01-01 custom "budget" "..." TRUE 4.30 USD
		"#;
			let (statements, errors) = parse_str(filename.clone(), src);

			print_errors(filename, src, errors);

			if let Some(statements) = statements {
				println!("{:#?}", statements);
			}
		}
	}
}
