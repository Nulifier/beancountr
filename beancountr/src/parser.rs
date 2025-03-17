use crate::core::directive::Directive;
use ariadne::{sources, Color, Fmt, Label, Report, ReportKind};
use chumsky::prelude::*;
use chumsky::{error::Simple, text, Parser};
use rust_decimal::prelude::*;
use std::ops::Range;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
	// Directives
	Open,
	Close,
	CommodityDirective,
	Transaction,
	Balance,
	Pad,
	Note,
	Document,
	Price,
	Event,
	Query,
	Custom,

	// Commands
	Option,
	Plugin,
	Include,
	PushTag,
	PopTag,

	// Identifiers
	Account(Vec<String>),
	Commodity(String),
	Capital(char), // Both flags and single letter commdities
	Tag(String),   // #[A-Za-z0-9-_/.]+
	Link(String),  // ^[A-Za-z0-9-_/.]+

	// Literals
	Date(u16, u16, u16),
	Decimal(Decimal), // ([0-9]+|[0-9][0-9,]+[0-9])(\.[0-9]*)?
	String(String),

	// Punctuation
	Pipe,       // Used for legacy payee, narration separator
	At,         // Price
	LeftCurly,  // Costs
	RightCurly, // Costs
	Comma,      // Separator in open directives
	Tilde,      // Used for local tolarances
	Plus,       // Artithmetic expressions
	Minus,      // Artithmetic expressions
	Slash,      // Artithmetic expressions
	LeftParen,  // Artithmetic expressions
	RightParen, // Artithmetic expressions
	Colon,      // Metadata
	Asterisk,   // Flags and arithmetic expressions

	Exclamation, // Flags
	Ampersand,   // Flags
	Hash,        // Flags
	Question,    // Flags
	Percent,     // Flags
}

/// Checks if a character is an uppercase unicode alphabetic character, or if
/// the script doesn't have the concept of uppercase, an alphabetic character.
fn is_uppercase_or_caseless(c: char) -> bool {
	c.is_uppercase() || (c.is_alphabetic() && !c.is_lowercase())
}

pub fn lexer() -> impl Parser<char, Vec<(Token, Range<usize>)>, Error = Simple<char>> {
	let directive = choice((
		text::keyword("open").map(|_| Token::Open),
		text::keyword("close").map(|_| Token::Close),
		text::keyword("commodity").map(|_| Token::CommodityDirective),
		text::keyword("txn").map(|_| Token::Transaction),
		text::keyword("balance").map(|_| Token::Balance),
		text::keyword("pad").map(|_| Token::Pad),
		text::keyword("note").map(|_| Token::Note),
		text::keyword("document").map(|_| Token::Document),
		text::keyword("price").map(|_| Token::Price),
		text::keyword("event").map(|_| Token::Event),
		text::keyword("query").map(|_| Token::Query),
		text::keyword("custom").map(|_| Token::Custom),
	))
	.padded();

	let command = choice((
		text::keyword("option").to(Token::Option),
		text::keyword("plugin").to(Token::Plugin),
		text::keyword("include").to(Token::Include),
		text::keyword("pushtag").to(Token::PushTag),
		text::keyword("poptag").to(Token::PopTag),
	))
	.padded();

	// ACCOUNTTYPE = ([A-Z]|{UTF-8-ONLY})([A-Za-z0-9\-]|{UTF-8-ONLY})*
	let account_type = filter(|c: &char| is_uppercase_or_caseless(*c))
		.chain(filter(|c: &char| c.is_alphanumeric() || *c == '-').repeated())
		.collect::<String>();
	// ACCOUNTNAME = ([A-Z0-9]|{UTF-8-ONLY})([A-Za-z0-9\-]|{UTF-8-ONLY})*
	let account_name = filter(|c: &char| is_uppercase_or_caseless(*c) || c.is_digit(10))
		.chain(filter(|c: &char| c.is_alphanumeric() || *c == '-').repeated())
		.collect::<String>();
	// {ACCOUNTTYPE}(:{ACCOUNTNAME})+
	let account = account_type
		.then_ignore(just(':'))
		.then(account_name.separated_by(just(':')).at_least(1))
		.padded()
		.map(move |(account_type, ref mut account_name)| {
			// Add the account type to the begining
			let mut parts = vec![account_type];
			parts.append(account_name);
			Token::Account(parts)
		})
		.labelled("account");

	let commodity_no_slash = filter(|c: &char| is_uppercase_or_caseless(*c))
		.chain(
			filter(|c: &char| {
				is_uppercase_or_caseless(*c)
					|| c.is_digit(10)
					|| *c == '.' || *c == '_'
					|| *c == '-' || *c == '\''
			})
			.repeated()
			.at_least(1),
		)
		.collect::<String>();
	let commodity_slash = just('/')
		.chain(
			filter(|c: &char| {
				is_uppercase_or_caseless(*c)
					|| c.is_digit(10)
					|| *c == '.' || *c == '_'
					|| *c == '-' || *c == '\''
			})
			.repeated()
			.at_least(1),
		)
		.collect::<String>();

	let commodity = commodity_no_slash
		.or(commodity_slash)
		.try_map(|s, span| {
			if s.ends_with(|c| is_uppercase_or_caseless(c) || c.is_digit(10)) {
				Ok(s)
			} else {
				Err(Simple::custom(
					span,
					"Commodities must end with an uppercase letter or digit",
				))
			}
		})
		.map(Token::Commodity)
		.padded();

	let capital = filter(|c: &char| is_uppercase_or_caseless(*c))
		.map(Token::Capital)
		.padded();

	let tag = just('#')
		.ignore_then(
			filter(|c: &char| {
				c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '/' || *c == '.'
			})
			.repeated()
			.at_least(1),
		)
		.collect::<String>()
		.map(Token::Tag)
		.padded();

	let link = just('^')
		.ignore_then(
			filter(|c: &char| {
				c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '/' || *c == '.'
			})
			.repeated()
			.at_least(1),
		)
		.collect::<String>()
		.map(Token::Link)
		.padded();

	let four_digits = filter(move |c: &char| c.is_digit(10))
		.repeated()
		.exactly(4)
		.collect::<String>()
		.map(|s| s.parse::<u16>().unwrap());
	let two_digits = filter(move |c: &char| c.is_digit(10))
		.repeated()
		.exactly(2)
		.collect::<String>()
		.map(|s| s.parse::<u16>().unwrap());

	let date = four_digits
		.then_ignore(just('-'))
		.then(two_digits)
		.then_ignore(just('-'))
		.then(two_digits)
		.or(four_digits
			.then_ignore(just('/'))
			.then(two_digits)
			.then_ignore(just('/'))
			.then(two_digits))
		.map(|((year, month), day)| Token::Date(year, month, day))
		.padded();

	let escape = just('\\').ignore_then(
		just('\\')
			.or(just('/'))
			.or(just('"'))
			.or(just('n').to('\n'))
			.or(just('r').to('\r'))
			.or(just('t').to('\t')),
	);

	let string = just('"')
		.ignore_then(
			filter(|c| (*c != '\\') && (*c != '"'))
				.or(escape)
				.repeated(),
		)
		.then_ignore(just('"'))
		.collect::<String>()
		.map(Token::String)
		.padded()
		.labelled("string");

	let integer = filter(|c: &char| c.is_digit(10))
		.chain(filter(|c: &char| c.is_digit(10) || *c == ',').repeated())
		.collect::<String>()
		.labelled("integer");
	let decimal = filter(|c: &char| c.is_digit(10))
		.chain(filter(|c: &char| c.is_digit(10) || *c == ',').repeated())
		.chain(just('.'))
		.chain(filter(|c: &char| c.is_digit(10)).repeated())
		.collect::<String>()
		.labelled("decimal");
	let number = integer
		.or(decimal)
		.try_map(|s, span| {
			Decimal::from_str(&s)
				.map_err(|e| Simple::custom(span, format!("Error parsing decimal: {}", e)))
		})
		.padded();

	let token = choice((
		directive,
		command,
		date,
		account,
		commodity,
		capital,
		tag,
		link,
		number,
		//text::decimal().map(|_| Token::Decimal),
		string,
		just(':').to(Token::Colon),
		just(',').to(Token::Comma),
		just('@').to(Token::At),
		just('!').to(Token::Exclamation),
		just('*').to(Token::Asterisk),
		just('(').to(Token::LeftParen),
		just(')').to(Token::RightParen),
		just('{').to(Token::LeftCurly),
		just('}').to(Token::RightCurly),
		just('/').to(Token::Slash),
		just('-').to(Token::Minus),
		just('+').to(Token::Plus),
		just('|').to(Token::Pipe),
		//just('#').to(Token::Hash),
		//just('^').to(Token::Caret),
		just('~').to(Token::Tilde),
	))
	.recover_with(skip_then_retry_until([]));

	let comment = just(';').then(take_until(just('\n'))).padded();

	token
		.map_with_span(|tok, span| (tok, span))
		.padded_by(comment.repeated())
		.padded()
		.repeated()
		.then_ignore(end())
}

#[derive(Debug)]
pub enum Statement {
	Option(String, String),
	Plugin(String, Option<String>),
	Include(String),
	Comment(String),
	Directive(Directive),

	// Testing only
	Date(u16, u16, u16),
	String(String),
}

// pub fn parser() -> impl Parser<char, Vec<Statement>, Error = Simple<char>> {
// 	let four_digits = filter(move |c: &char| c.is_digit(10))
// 		.repeated()
// 		.exactly(4)
// 		.collect::<String>()
// 		.map(|s| s.parse::<u16>().unwrap());
// 	let two_digits = filter(move |c: &char| c.is_digit(10))
// 		.repeated()
// 		.exactly(2)
// 		.collect::<String>()
// 		.map(|s| s.parse::<u16>().unwrap());

// 	let date = four_digits
// 		.then_ignore(just('-'))
// 		.then(two_digits)
// 		.then_ignore(just('-'))
// 		.then(two_digits)
// 		.or(four_digits
// 			.then_ignore(just('/'))
// 			.then(two_digits)
// 			.then_ignore(just('/'))
// 			.then(two_digits))
// 		.map(|((year, month), day)| Statement::Date(year, month, day))
// 		.padded();

// 	let option = text::keyword("option")
// 		.ignored()
// 		.then_ignore(text::whitespace())
// 		.then(string)
// 		.then_ignore(text::whitespace())
// 		.then(string)
// 		.map(|((_, key), value)| Statement::Option(key, value))
// 		.padded();

// 	let plugin = text::keyword("plugin")
// 		.ignored()
// 		.then_ignore(text::whitespace())
// 		.then(string)
// 		.then_ignore(text::whitespace())
// 		.then(string.or_not())
// 		.map(|((_, key), value)| Statement::Plugin(key, value))
// 		.padded();

// 	let include = text::keyword("include")
// 		.ignored()
// 		.then_ignore(text::whitespace())
// 		.then(string)
// 		.map(|(_, path)| Statement::Include(path))
// 		.padded();

// 	choice((option, plugin, include, date))
// 		.repeated()
// 		.then_ignore(end())
// }

pub fn parse_str(filename: Rc<str>, src: &str) {
	let (tokens, errs) = lexer().parse_recovery(src);

	if let Some(tokens) = tokens {
		println!("Tokens:");
		for token in tokens {
			println!("- {:?}", token);
		}
	}

	errs.into_iter()
		.map(|e| e.map(|c| c.to_string()))
		.for_each(|e| {
			let report = Report::build(ReportKind::Error, (filename.clone(), e.span()));

			let report = match e.reason() {
				chumsky::error::SimpleReason::Unclosed { span, delimiter } => report
					.with_message(format!(
						"Unclosed delimiter: {:?}",
						delimiter.fg(Color::Yellow)
					))
					.with_label(
						Label::new((filename.clone(), span.clone()))
							.with_message(format!(
								"Unclosed delimiter: {:?}",
								delimiter.fg(Color::Yellow)
							))
							.with_color(Color::Yellow),
					)
					.with_label(
						Label::new((filename.clone(), e.span()))
							.with_message(format!(
								"Must be closed before this {}",
								e.found()
									.unwrap_or(&"end of file".to_string())
									.fg(Color::Red)
							))
							.with_color(Color::Red),
					),
				chumsky::error::SimpleReason::Unexpected => report
					.with_message(format!(
						"{}, expected {}, while parsing {}",
						if e.found().is_some() {
							"Unexpected token found in input"
						} else {
							"Unexpected end of input"
						},
						if e.expected().len() == 0 {
							"something else".to_string()
						} else {
							e.expected()
								.map(|expected| match expected {
									Some(expected) => expected.to_string(),
									None => "end of input".to_string(),
								})
								.collect::<Vec<_>>()
								.join(", ")
						},
						e.label().unwrap_or(&"input".to_string()),
					))
					.with_label(
						Label::new((filename.clone(), e.span()))
							.with_message(format!(
								"Unexpected token {}",
								e.found()
									.unwrap_or(&"end of file".to_string())
									.fg(Color::Red)
							))
							.with_color(Color::Red),
					),
				chumsky::error::SimpleReason::Custom(msg) => report.with_message(msg).with_label(
					Label::new((filename.clone(), e.span()))
						.with_message(format!("{}", msg.fg(Color::Red)))
						.with_color(Color::Red),
				),
			};

			report
				.finish()
				.print(sources([(filename.clone(), src)]))
				.unwrap();
		});
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_uppercase() {
		// Basic ascii
		assert!(is_uppercase_or_caseless('A'));
		assert!(is_uppercase_or_caseless('Z'));
		assert!(!is_uppercase_or_caseless('a'));
		assert!(!is_uppercase_or_caseless('z'));

		// Non-alphabetic
		assert!(!is_uppercase_or_caseless('0'));
		assert!(!is_uppercase_or_caseless(' '));

		// Unicode
		assert!(is_uppercase_or_caseless('Δ'));
		assert!(!is_uppercase_or_caseless('δ'));

		// Caseless
		assert!(is_uppercase_or_caseless('中'));
	}

	#[test]
	fn test_lexer_directives() {
		let src = r#"
			open close commodity txn balance pad note
			document price event query custom
		"#;

		let tokens: Vec<Token> = lexer()
			.parse(src)
			.unwrap()
			.into_iter()
			.map(|(tok, _)| tok)
			.collect();
		assert_eq!(
			tokens,
			vec![
				Token::Open,
				Token::Close,
				Token::CommodityDirective,
				Token::Transaction,
				Token::Balance,
				Token::Pad,
				Token::Note,
				Token::Document,
				Token::Price,
				Token::Event,
				Token::Query,
				Token::Custom,
			]
		);
	}

	#[test]
	fn test_lexer_commands() {
		let src = r#"
			option plugin include pushtag poptag
		"#;

		let tokens: Vec<Token> = lexer()
			.parse(src)
			.unwrap()
			.into_iter()
			.map(|(tok, _)| tok)
			.collect();
		assert_eq!(
			tokens,
			vec![
				Token::Option,
				Token::Plugin,
				Token::Include,
				Token::PushTag,
				Token::PopTag,
			]
		);
	}

	#[test]
	fn test_lexer_identifiers() {
		let src = r#"
			Assets:US:B-of-A:Checking
			Assets:CAD:TD:1233456
			Λiabilities:Credit
			AAPL
			NT.TO
			TLT_040921C144
			/6J
			/NQH21
			/NQH21_QNEG21C13100
			C
			Λ
			中
			#This-is/a_tag.1
			^This-is/a_link.1
		"#;

		let tokens: Vec<Token> = lexer()
			.parse(src)
			.unwrap()
			.into_iter()
			.map(|(tok, _)| tok)
			.collect();
		assert_eq!(
			tokens,
			vec![
				Token::Account(vec![
					"Assets".to_string(),
					"US".to_string(),
					"B-of-A".to_string(),
					"Checking".to_string(),
				]),
				Token::Account(vec![
					"Assets".to_string(),
					"CAD".to_string(),
					"TD".to_string(),
					"1233456".to_string(),
				]),
				Token::Account(vec!["Λiabilities".to_string(), "Credit".to_string(),]),
				Token::Commodity("AAPL".to_string()),
				Token::Commodity("NT.TO".to_string()),
				Token::Commodity("TLT_040921C144".to_string()),
				Token::Commodity("/6J".to_string()),
				Token::Commodity("/NQH21".to_string()),
				Token::Commodity("/NQH21_QNEG21C13100".to_string()),
				Token::Capital('C'),
				Token::Capital('Λ'),
				Token::Capital('中'),
				Token::Tag("This-is/a_tag.1".to_string()),
				Token::Link("This-is/a_link.1".to_string()),
			]
		);
	}

	#[test]
	fn test_lexer_literals() {
		let src = r#"
			2025-03-01
			2025/03/01
			"Hello, World!"
			"Special chars: \n\t\r\"/\\"
			"Hello\tWorld!"
			"Unicode: κόσμε"
			"Emojis: 😁"
			2.0200
			58979323846264338.32
			65535
			0.00000097
			-3.14
			1,234,567.89
		"#;

		let tokens: Vec<Token> = lexer()
			.parse(src)
			.unwrap()
			.into_iter()
			.map(|(tok, _)| tok)
			.collect();
		assert_eq!(
			tokens,
			vec![
				Token::Date(2025, 3, 1),
				Token::Date(2025, 3, 1),
				Token::String("Hello, World!".to_string()),
				Token::String("Special chars: \n\t\r\"/\\".to_string()),
				Token::String("Hello\tWorld!".to_string()),
				Token::String("Unicode: κόσμε".to_string()),
				Token::String("Emojis: 😁".to_string()),
				Token::Decimal(Decimal::from_str("2.0200").unwrap()),
				Token::Decimal(Decimal::from_str("58979323846264338.32").unwrap()),
				Token::Decimal(Decimal::from_str("65535").unwrap()),
				Token::Decimal(Decimal::from_str("0.00000097").unwrap()),
				Token::Decimal(Decimal::from_str("-3.14").unwrap()),
				Token::Decimal(Decimal::from_str("1234567.89").unwrap()),
			]
		);
	}
}
