use std::rc::Rc;

use beancountr::parser::parse_str;
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

	//let src = "2025-03-01";

	match &cli.command {
		Commands::Test => parse_str(
			Rc::from("filename.beancount"),
			r#"
2.0200
58979323846264338.32
65535
0.00000097
-3.14
1,234,567.89
"#,
		),
	}
}
