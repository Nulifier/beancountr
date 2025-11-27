use crate::core::directive::{Directive, DirectiveKind, Metadata, MetadataMap, Posting};
use crate::core::types::{Account, Amount, Commodity};
use ariadne::{sources, Color, Fmt, Label, Report, ReportKind};
use chrono::{Datelike, NaiveDate};
use chumsky::{error::Simple, text, Parser};
use chumsky::{prelude::*, Stream};
use rust_decimal::prelude::*;
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::hash::Hash;
use std::ops::Range;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

	// Literals
	Date(NaiveDate),
	Decimal(Decimal), // ([0-9]+|[0-9][0-9,]+[0-9])(\.[0-9]*)?
	String(String),
	Bool(bool), // true, false
	Null,       // None

	// Identifiers
	Account(Vec<String>),
	Commodity(String),
	Capital(char), // Both flags and single letter commdities
	Tag(String),   // #[A-Za-z0-9-_/.]+
	Link(String),  // ^[A-Za-z0-9-_/.]+
	Key(String),   // [a-z][a-zA-Z0-9\-_]*:

	// Punctuation
	Pipe,          // Used for legacy payee, narration separator
	AtAt,          // Price
	At,            // Price
	LeftCurlCurl,  // Costs
	LeftCurl,      // Costs
	RightCurlCurl, // Costs
	RightCurl,     // Costs
	Comma,         // Separator in open directives
	Tilde,         // Used for local tolerances
	Plus,          // Arithmetic expressions
	Minus,         // Arithmetic expressions
	Slash,         // Arithmetic expressions
	LeftParen,     // Arithmetic expressions
	RightParen,    // Arithmetic expressions
	Asterisk,      // Flags and arithmetic expressions

	Exclamation, // Flags
	Ampersand,   // Flags
	Hash,        // Flags
	Question,    // Flags
	Percent,     // Flags

	Newline,
}

impl fmt::Display for Token {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Token::Open => write!(f, "open"),
			Token::Close => write!(f, "close"),
			Token::CommodityDirective => write!(f, "commodity"),
			Token::Transaction => write!(f, "txn"),
			Token::Balance => write!(f, "balance"),
			Token::Pad => write!(f, "pad"),
			Token::Note => write!(f, "note"),
			Token::Document => write!(f, "document"),
			Token::Price => write!(f, "price"),
			Token::Event => write!(f, "event"),
			Token::Query => write!(f, "query"),
			Token::Custom => write!(f, "custom"),
			Token::Option => write!(f, "option"),
			Token::Plugin => write!(f, "plugin"),
			Token::Include => write!(f, "include"),
			Token::PushTag => write!(f, "pushtag"),
			Token::PopTag => write!(f, "poptag"),
			Token::Date(d) => write!(f, "{}-{:02}-{:02}", d.year(), d.month0() + 1, d.day0() + 1),
			Token::Decimal(d) => write!(f, "{}", d),
			Token::String(s) => write!(f, "\"{}\"", s),
			Token::Bool(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
			Token::Null => write!(f, "NULL"),
			Token::Account(parts) => write!(f, "{}", parts.join(":")),
			Token::Commodity(s) => write!(f, "{}", s),
			Token::Capital(c) => write!(f, "{}", c),
			Token::Tag(s) => write!(f, "#{}", s),
			Token::Link(s) => write!(f, "^{}", s),
			Token::Key(s) => write!(f, "{}:", s),
			Token::Pipe => write!(f, "|"),
			Token::AtAt => write!(f, "@@"),
			Token::At => write!(f, "@"),
			Token::LeftCurlCurl => write!(f, "{{{{"),
			Token::LeftCurl => write!(f, "{{"),
			Token::RightCurlCurl => write!(f, "}}}}"),
			Token::RightCurl => write!(f, "}}"),
			Token::Comma => write!(f, ","),
			Token::Tilde => write!(f, "~"),
			Token::Plus => write!(f, "+"),
			Token::Minus => write!(f, "-"),
			Token::Slash => write!(f, "/"),
			Token::LeftParen => write!(f, "("),
			Token::RightParen => write!(f, ")"),
			Token::Asterisk => write!(f, "*"),
			Token::Exclamation => write!(f, "!"),
			Token::Ampersand => write!(f, "&"),
			Token::Hash => write!(f, "#"),
			Token::Question => write!(f, "?"),
			Token::Percent => write!(f, "%"),
			Token::Newline => write!(f, "\\n"),
		}
	}
}

/// Checks if a character is an uppercase unicode alphabetic character, or if
/// the script doesn't have the concept of uppercase, an alphabetic character.
fn is_uppercase_or_caseless(c: char) -> bool {
	c.is_uppercase() || (c.is_alphabetic() && !c.is_lowercase())
}

pub fn lexer() -> impl Parser<char, Vec<(Token, Range<usize>)>, Error = Simple<char>> {
	let safe_whitespace = one_of(" \r\t").ignored().repeated();

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
	.boxed();

	let command = choice((
		text::keyword("option").to(Token::Option),
		text::keyword("plugin").to(Token::Plugin),
		text::keyword("include").to(Token::Include),
		text::keyword("pushtag").to(Token::PushTag),
		text::keyword("poptag").to(Token::PopTag),
	))
	.boxed();

	let four_digits = filter(move |c: &char| c.is_digit(10))
		.repeated()
		.exactly(4)
		.collect::<String>()
		.map(|s| s.parse::<i32>().unwrap());
	let two_digits = filter(move |c: &char| c.is_digit(10))
		.repeated()
		.exactly(2)
		.collect::<String>()
		.map(|s| s.parse::<u32>().unwrap());

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
		.try_map(|((year, month), day), span| {
			NaiveDate::from_ymd_opt(year, month, day)
				.ok_or_else(|| Simple::custom(span, "Invalid date"))
				.map(Token::Date)
		})
		.boxed();

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
		.labelled("string")
		.boxed();

	let bool_ = choice((
		text::keyword("TRUE").to(Token::Bool(true)),
		text::keyword("FALSE").to(Token::Bool(false)),
	));

	let null = text::keyword("NULL").to(Token::Null);

	let number = filter(|c: &char| c.is_digit(10))
		.chain(filter(|c: &char| c.is_digit(10) || *c == ',').repeated())
		.chain::<char, _, _>(
			just('.')
				.chain(filter(|c: &char| c.is_digit(10)).repeated())
				.or_not()
				.flatten(),
		)
		.collect::<String>()
		.try_map(|s, span| {
			Decimal::from_str(&s.replace(",", ""))
				.map_err(|e| Simple::custom(span, format!("Error parsing decimal: {}", e)))
		})
		.map(Token::Decimal)
		.boxed();

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
		.map(move |(account_type, ref mut account_name)| {
			// Add the account type to the begining
			let mut parts = vec![account_type];
			parts.append(account_name);
			Token::Account(parts)
		})
		.labelled("account")
		.boxed();

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
		.collect::<String>()
		.try_map(|s, span| {
			if s.contains(|c: char| c.is_alphabetic()) {
				Ok(s)
			} else {
				Err(Simple::custom(
					span,
					"If a commodity begins with a slash, the following pattern has to include at least one letter",
				))
			}
		});

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
		.boxed();

	let capital = filter(|c: &char| is_uppercase_or_caseless(*c))
		.map(Token::Capital)
		.boxed();

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
		.boxed();

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
		.boxed();

	let key = filter(|c: &char| c.is_lowercase())
		.chain(filter(|c: &char| c.is_alphanumeric() || *c == '-' || *c == '_').repeated())
		.then_ignore(just(':'))
		.collect::<String>()
		.map(Token::Key)
		.boxed();

	let punctuation = choice((
		just(',').to(Token::Comma),
		just("@@").to(Token::AtAt),
		just('@').to(Token::At),
		just('!').to(Token::Exclamation),
		just('*').to(Token::Asterisk),
		just('(').to(Token::LeftParen),
		just(')').to(Token::RightParen),
		just("{{").to(Token::LeftCurlCurl),
		just('{').to(Token::LeftCurl),
		just("}}").to(Token::RightCurlCurl),
		just('}').to(Token::RightCurl),
		just('/').to(Token::Slash),
		just('-').to(Token::Minus),
		just('+').to(Token::Plus),
		just('|').to(Token::Pipe),
		just('~').to(Token::Tilde),
	))
	.boxed();

	let newline = text::newline().map(|_| Token::Newline).boxed();

	let token = choice((
		directive,
		command,
		date,
		number,
		string,
		bool_,
		null,
		account,
		commodity,
		capital,
		tag,
		link,
		key,
		punctuation,
		newline,
	))
	.recover_with(skip_then_retry_until([]));

	let comment = just(';').then(take_until(just('\n'))).padded();

	token
		.map_with_span(|tok, span| (tok, span))
		.padded_by(safe_whitespace)
		.padded_by(comment.repeated())
		.repeated()
		.then_ignore(end())
}

pub fn expr_parser() -> impl Parser<Token, Decimal, Error = Simple<Token>> {
	recursive(|expr| {
		let number = select! {
			Token::Decimal(d) => d,
		};

		let atom = number.or(expr.delimited_by(just(Token::LeftParen), just(Token::RightParen)));

		// ( ) * / - +

		let unary = just(Token::Minus)
			.or(just(Token::Plus))
			.repeated()
			.then(atom)
			.foldr(|op, n| match op {
				Token::Minus => -n,
				Token::Plus => n,
				_ => unreachable!(),
			})
			.boxed();

		let product = unary
			.clone()
			.then(
				just(Token::Asterisk)
					.or(just(Token::Slash))
					.then(unary)
					.repeated(),
			)
			.foldl(|lhs, (op, rhs)| match op {
				Token::Asterisk => lhs * rhs,
				Token::Slash => lhs / rhs,
				_ => unreachable!(),
			})
			.boxed();

		let sum = product
			.clone()
			.then(
				just(Token::Plus)
					.or(just(Token::Minus))
					.then(product)
					.repeated(),
			)
			.foldl(|lhs, (op, rhs)| match op {
				Token::Plus => lhs + rhs,
				Token::Minus => lhs - rhs,
				_ => unreachable!(),
			});

		sum
	})
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
	Option(String, String),
	Plugin(String, Option<String>),
	Include(String),
	Directive(Directive),

	// Testing only
	Date(u16, u16, u16),
	String(String),
}

pub fn parser<F: Fn(usize) -> usize>(
	filename: Rc<str>,
	line_lookup: F,
) -> impl Parser<Token, Vec<Statement>, Error = Simple<Token>> {
	// Helpers

	let end_of_line = choice((just(Token::Newline).to(()), end())).boxed();

	let string = (select! {
		Token::String(s) => s,
	})
	.labelled("string");

	let date = (select! {
		Token::Date(date) => date,
	})
	.labelled("date");

	let account = (select! {
		Token::Account(parts) => Account::from(parts),
	})
	.labelled("account");

	let commodity = select! {
		Token::Commodity(s) => s.parse::<Commodity>().unwrap(),
	};

	let commodity_list = commodity.separated_by(just(Token::Comma));

	let amount = expr_parser()
		.then(commodity)
		.map(|(number, commodity)| Amount::new(number, commodity))
		.boxed();

	let amount_tolerance = amount.clone().map(|amount| (amount, None)).or(expr_parser()
		.then_ignore(just(Token::Tilde))
		.then(expr_parser())
		.then(commodity)
		.map(|((number, tolerance), commodity)| (Amount::new(number, commodity), Some(tolerance)))
		.boxed());

	let tag = select! {
		Token::Tag(s) => s,
	};

	let bool_ = select! {
		Token::Bool(b) => b,
	};

	let metadata_value = string
		.map(Metadata::String)
		.or(account.map(Metadata::Account))
		.or(date.map(Metadata::Date))
		.or(commodity.map(Metadata::Commodity))
		.or(tag.map(Metadata::Tags))
		.or(bool_.map(Metadata::Bool))
		.or(just(Token::Null).to(Metadata::None))
		.or(amount.clone().map(Metadata::Amount))
		.or(expr_parser().map(Metadata::Number))
		.boxed();

	let metadata_line = select! {
		Token::Key(key) => key,
	}
	.then(metadata_value.clone())
	.then_ignore(end_of_line.clone());

	let metadata = metadata_line
		.clone()
		.repeated()
		.map(|pairs| pairs.into_iter().collect())
		.boxed();

	let tag_or_link_token = select! {
		Token::Tag(s) => Token::Tag(s),
		Token::Link(s) => Token::Link(s),
	};

	let tags_links = tag_or_link_token
		.repeated()
		.map(|tags_links| {
			let mut tags = HashSet::new();
			let mut links = HashSet::new();
			for tag_link in tags_links {
				match tag_link {
					Token::Tag(tag) => {
						tags.insert(tag);
					}
					Token::Link(link) => {
						links.insert(link);
					}
					_ => {}
				}
			}
			(tags, links)
		})
		.boxed();

	// Statements

	let option = just(Token::Option)
		.ignore_then(string)
		.then(string)
		.then_ignore(end_of_line.clone())
		.map(|(key, value)| Statement::Option(key, value))
		.boxed();

	let plugin = just(Token::Plugin)
		.ignore_then(string)
		.then(string.or_not())
		.then_ignore(end_of_line.clone())
		.map(|(name, arg)| Statement::Plugin(name, arg))
		.boxed();

	let include = just(Token::Include)
		.ignore_then(string)
		.then_ignore(end_of_line.clone())
		.map(Statement::Include)
		.boxed();

	let open_directive = date
		.then_ignore(just(Token::Open))
		.then(account)
		.then(commodity_list)
		.then(string.or_not())
		.then_ignore(end_of_line.clone())
		.then(metadata.clone().or_not())
		.map(|((((date, account), commodities), booking_method), meta)| {
			Statement::Directive(Directive::new(
				date,
				DirectiveKind::Open(account, commodities, booking_method.map(|s| s.into())),
				meta.unwrap_or_default(),
			))
		})
		.boxed();

	let close_directive = date
		.then_ignore(just(Token::Close))
		.then(account)
		.then_ignore(end_of_line.clone())
		.then(metadata.clone().or_not())
		.map(|((date, account), meta)| {
			Statement::Directive(Directive::new(
				date,
				DirectiveKind::Close(account),
				meta.unwrap_or_default(),
			))
		})
		.boxed();

	let commodity_directive = date
		.then_ignore(just(Token::CommodityDirective))
		.then(commodity)
		.then_ignore(end_of_line.clone())
		.then(metadata.clone().or_not())
		.map(|((date, commodity), meta)| {
			Statement::Directive(Directive::new(
				date,
				DirectiveKind::Commodity(commodity),
				meta.unwrap_or_default(),
			))
		})
		.boxed();

	let pad_directive = date
		.then_ignore(just(Token::Pad))
		.then(account)
		.then(account)
		.then_ignore(end_of_line.clone())
		.then(metadata.clone().or_not())
		.map(|(((date, account), source_account), meta)| {
			Statement::Directive(Directive::new(
				date,
				DirectiveKind::Pad {
					account,
					source_account,
				},
				meta.unwrap_or_default(),
			))
		})
		.boxed();

	let balance_directive = date
		.then_ignore(just(Token::Balance))
		.then(account)
		.then(amount_tolerance)
		.then_ignore(end_of_line.clone())
		.then(metadata.clone().or_not())
		.map(|(((date, account), (amount, tolerance)), meta)| {
			Statement::Directive(Directive::new(
				date,
				DirectiveKind::Balance {
					account,
					amount,
					tolerance,
					diff_amount: None,
				},
				meta.unwrap_or_default(),
			))
		})
		.boxed();

	let capital = select! {
		Token::Capital(c) => c,
	};

	// Valid flags: !&#?%* and Capitals
	let flag = just(Token::Transaction)
		.to('*')
		.or(just(Token::Exclamation).to('!'))
		.or(just(Token::Ampersand).to('&'))
		.or(just(Token::Hash).to('#'))
		.or(just(Token::Question).to('?'))
		.or(just(Token::Percent).to('%'))
		.or(just(Token::Asterisk).to('*'))
		.or(capital)
		.boxed();

	enum PostingOrMetadata {
		Posting(Posting),
		Metadata(String, Metadata),
	}

	let posting = flag
		.clone()
		.or_not()
		.then(account)
		.then_ignore(end_of_line.clone())
		.map(|(flag, account)| {
			Posting::new(account, None, None, None, flag, MetadataMap::default())
		});

	let posting_or_metadata = posting
		.map(PostingOrMetadata::Posting)
		.or(metadata_line
			.clone()
			.map(|(key, value)| PostingOrMetadata::Metadata(key, value)))
		.boxed();

	let transaction_directive = date
		.then(flag)
		.then(string.or_not())
		.then(string.or_not())
		.then_ignore(end_of_line.clone())
		.then(posting_or_metadata.repeated())
		.map(|((((date, flag), str_a), str_b), other)| {
			// If both are present, the first is the payee and the second is the narration
			// If only the first is present, it is the narration
			let (payee, narration) = match (str_a, str_b) {
				(Some(a), Some(b)) => (Some(a), Some(b)),
				(Some(a), None) => (None, Some(a)),
				_ => (None, None),
			};

			let mut tx_meta = MetadataMap::default();
			let mut postings = vec![];
			let mut current_posting = None;
			for item in other {
				match item {
					// If the item is a posting, we save the last one and set this one to current
					PostingOrMetadata::Posting(p) => {
						if let Some(current) = current_posting.take() {
							postings.push(current);
						}
						current_posting = Some(p);
					}
					// If the item is a metadata, we insert it into the current posting
					PostingOrMetadata::Metadata(k, v) => {
						if let Some(current) = current_posting.as_mut() {
							// If there is a current posting, we insert the metadata into it
							current.meta.insert(k, v);
						} else {
							// If there is no current posting, we insert it into the transaction metadata
							tx_meta.insert(k, v);
						}
					}
				}
			}

			// If there is a current posting, we push it to the list
			if let Some(current) = current_posting.take() {
				postings.push(current);
			}

			Statement::Directive(Directive::new(
				date,
				DirectiveKind::Transaction {
					flag: Some(flag),
					payee,
					narration,
					tags: HashSet::default(),
					links: HashSet::default(),
					postings,
				},
				tx_meta,
			))
		});

	let note_directive = date
		.then_ignore(just(Token::Note))
		.then(account)
		.then(string)
		.then(tags_links.clone())
		.then_ignore(end_of_line.clone())
		.then(metadata.clone().or_not())
		.map(|((((date, account), comment), (tags, links)), meta)| {
			Statement::Directive(Directive::new(
				date,
				DirectiveKind::Note {
					account,
					comment,
					tags,
					links,
				},
				meta.unwrap_or_default(),
			))
		})
		.boxed();

	let event_directive = date
		.then_ignore(just(Token::Event))
		.then(string)
		.then(string)
		.then_ignore(end_of_line.clone())
		.then(metadata.clone().or_not())
		.map(|(((date, kind), description), meta)| {
			Statement::Directive(Directive::new(
				date,
				DirectiveKind::Event { kind, description },
				meta.unwrap_or_default(),
			))
		})
		.boxed();

	let query_directive = date
		.then_ignore(just(Token::Query))
		.then(string)
		.then(string)
		.then_ignore(end_of_line.clone())
		.then(metadata.clone().or_not())
		.map(|(((date, name), query), meta)| {
			Statement::Directive(Directive::new(
				date,
				DirectiveKind::Query { name, query },
				meta.unwrap_or_default(),
			))
		})
		.boxed();

	let price_directive = date
		.then_ignore(just(Token::Price))
		.then(commodity)
		.then(amount)
		.then_ignore(end_of_line.clone())
		.then(metadata.clone().or_not())
		.map(|(((date, commodity), amount), meta)| {
			Statement::Directive(Directive::new(
				date,
				DirectiveKind::Price { commodity, amount },
				meta.unwrap_or_default(),
			))
		})
		.boxed();

	let document_directive = date
		.then_ignore(just(Token::Document))
		.then(account)
		.then(string)
		.then(tags_links)
		.then_ignore(end_of_line.clone())
		.then(metadata.clone().or_not())
		.map(|((((date, account), filename), (tags, links)), meta)| {
			Statement::Directive(Directive::new(
				date,
				DirectiveKind::Document {
					account,
					filename,
					tags,
					links,
				},
				meta.unwrap_or_default(),
			))
		})
		.boxed();

	let custom_directive = date
		.then_ignore(just(Token::Custom))
		.then(string)
		.then(metadata_value.repeated())
		.then_ignore(end_of_line.clone())
		.then(metadata.clone().or_not())
		.map(|(((date, kind), values), meta)| {
			Statement::Directive(Directive::new(
				date,
				DirectiveKind::Custom { kind, values },
				meta.unwrap_or_default(),
			))
		});

	let directive = choice((
		open_directive,
		close_directive,
		commodity_directive,
		pad_directive,
		balance_directive,
		transaction_directive,
		note_directive,
		event_directive,
		query_directive,
		price_directive,
		document_directive,
		custom_directive,
	))
	.map_with_span(move |stmt, span: Range<usize>| match stmt {
		Statement::Directive(d) => {
			let mut meta = d.meta;
			meta.insert(
				"filename".to_string(),
				Metadata::String(filename.to_string()),
			);
			meta.insert(
				"lineno".to_string(),
				Metadata::Number(Decimal::from(line_lookup(span.start))),
			);
			Statement::Directive(Directive::new(d.date, d.kind, meta))
		}
		_ => stmt,
	});

	let statement = choice((option, plugin, include, directive));

	statement
		.padded_by(just(Token::Newline).repeated())
		.repeated()
		.then_ignore(end())
}

/// Parses a string and returns a vector of statements and a vector of errors.
pub fn parse_str(filename: Rc<str>, src: &str) -> (Option<Vec<Statement>>, Vec<Simple<String>>) {
	// Create a line number lookup table
	let mut line_map = BTreeMap::new();
	let mut line = 1;
	for (i, _) in src.match_indices('\n') {
		line_map.insert(i, line);
		line += 1;
	}

	let line_lookup = |pos: usize| -> usize {
		// Get the line number for the given position
		line_map
			.range(..=pos)
			.next_back()
			.map(|(_, &line)| line)
			.unwrap_or(1)
	};

	let (tokens, errs) = lexer().parse_recovery(src);

	if let Some(tokens) = tokens.clone() {
		println!("Tokens:");
		for (token, _) in tokens {
			println!("- {:?}", token);
		}
	}

	let (statements, parse_errs) = if let Some(tokens) = tokens {
		let len = src.chars().count();
		let (statements, parse_errs) = parser(filename, line_lookup)
			.parse_recovery(Stream::from_iter(len..len + 1, tokens.into_iter()));

		(statements, parse_errs)
	} else {
		(None, Vec::new())
	};

	(
		statements,
		errs.into_iter()
			.map(|e| e.map(|c| c.to_string()))
			.chain(parse_errs.into_iter().map(|e| e.map(|tok| tok.to_string())))
			.collect(),
	)
}

/// Prints the errors to the console
/// using the `ariadne` crate for better formatting.
pub fn print_errors(filename: Rc<str>, src: &str, errors: Vec<Simple<String>>) {
	errors.into_iter().for_each(|e| {
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
	use std::collections::{HashMap, HashSet};

	use crate::core::types::Amount;

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
				Token::Newline,
				Token::Open,
				Token::Close,
				Token::CommodityDirective,
				Token::Transaction,
				Token::Balance,
				Token::Pad,
				Token::Note,
				Token::Newline,
				Token::Document,
				Token::Price,
				Token::Event,
				Token::Query,
				Token::Custom,
				Token::Newline,
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
				Token::Newline,
				Token::Option,
				Token::Plugin,
				Token::Include,
				Token::PushTag,
				Token::PopTag,
				Token::Newline,
			]
		);
	}

	#[test]
	fn test_lexer_literals() {
		let src = r#"
			2025-03-01 2025/03/01 "Hello, World!" "Special chars: \n\t\r\"/\\"
			"Hello\tWorld!" "Unicode: κόσμε" "Emojis: 😁" 2.0200
			58979323846264338.32 65535 0.00000097 -3.14 1,234,567.89
			TRUE FALSE NULL
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
				Token::Newline,
				Token::Date(NaiveDate::from_str("2025-03-01").unwrap()),
				Token::Date(NaiveDate::from_str("2025-03-01").unwrap()),
				Token::String("Hello, World!".to_string()),
				Token::String("Special chars: \n\t\r\"/\\".to_string()),
				Token::Newline,
				Token::String("Hello\tWorld!".to_string()),
				Token::String("Unicode: κόσμε".to_string()),
				Token::String("Emojis: 😁".to_string()),
				Token::Decimal(Decimal::from_str("2.0200").unwrap()),
				Token::Newline,
				Token::Decimal(Decimal::from_str("58979323846264338.32").unwrap()),
				Token::Decimal(Decimal::from_str("65535").unwrap()),
				Token::Decimal(Decimal::from_str("0.00000097").unwrap()),
				Token::Minus,
				Token::Decimal(Decimal::from_str("3.14").unwrap()),
				Token::Decimal(Decimal::from_str("1234567.89").unwrap()),
				Token::Newline,
				Token::Bool(true),
				Token::Bool(false),
				Token::Null,
				Token::Newline,
			]
		);
	}

	#[test]
	fn test_lexer_identifiers() {
		let src = r#"
			Assets:US:B-of-A:Checking Assets:CAD:TD:1233456 Λiabilities:Credit
			AAPL NT.TO TLT_040921C144 /6J /NQH21 /NQH21_QNEG21C13100 C Λ 中
			#This-is/a_tag.1 ^This-is/a_link.1 key: "value"
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
				Token::Newline,
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
				Token::Newline,
				Token::Commodity("AAPL".to_string()),
				Token::Commodity("NT.TO".to_string()),
				Token::Commodity("TLT_040921C144".to_string()),
				Token::Commodity("/6J".to_string()),
				Token::Commodity("/NQH21".to_string()),
				Token::Commodity("/NQH21_QNEG21C13100".to_string()),
				Token::Capital('C'),
				Token::Capital('Λ'),
				Token::Capital('中'),
				Token::Newline,
				Token::Tag("This-is/a_tag.1".to_string()),
				Token::Link("This-is/a_link.1".to_string()),
				Token::Key("key".to_string()),
				Token::String("value".to_string()),
				Token::Newline,
			]
		);
	}

	#[test]
	fn test_lexer_punctuation() {
		let src = ", @@ @ ! * ( ) { } / - + | ~";

		let tokens: Vec<Token> = lexer()
			.parse(src)
			.unwrap()
			.into_iter()
			.map(|(tok, _)| tok)
			.collect();
		assert_eq!(
			tokens,
			vec![
				Token::Comma,
				Token::AtAt,
				Token::At,
				Token::Exclamation,
				Token::Asterisk,
				Token::LeftParen,
				Token::RightParen,
				Token::LeftCurl,
				Token::RightCurl,
				Token::Slash,
				Token::Minus,
				Token::Plus,
				Token::Pipe,
				Token::Tilde,
			]
		);
	}

	#[test]
	fn test_expr_parser() {
		fn parse_expr(src: &str) -> Decimal {
			let tokens = lexer().parse(src).unwrap();
			expr_parser()
				.parse(Stream::from_iter(
					src.chars().count()..src.chars().count() + 1,
					tokens.into_iter(),
				))
				.unwrap()
		}

		assert_eq!(parse_expr("1"), Decimal::from_str("1").unwrap());
		assert_eq!(parse_expr("+1"), Decimal::from_str("1").unwrap());
		assert_eq!(parse_expr("-1"), Decimal::from_str("-1").unwrap());
		assert_eq!(parse_expr("2*3"), Decimal::from_str("6").unwrap());
		assert_eq!(parse_expr("6/3"), Decimal::from_str("2").unwrap());
		assert_eq!(parse_expr("1 +2"), Decimal::from_str("3").unwrap());
		assert_eq!(parse_expr("3 * 4 + 2"), Decimal::from_str("14").unwrap());
		assert_eq!(
			parse_expr("3.0 * (4.00 + 2)"),
			Decimal::from_str("18.00").unwrap()
		);
		assert_eq!(parse_expr("-4 + 2"), Decimal::from_str("-2").unwrap());
		assert_eq!(parse_expr("-(4 + 2)"), Decimal::from_str("-6").unwrap());
	}

	#[test]
	fn test_parser() {
		let filename: Rc<str> = Rc::from("test");
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

		let date = NaiveDate::from_str("2025-01-01").unwrap();

		let (statements, _errors) = parse_str(filename.clone(), src);

		assert_eq!(
			statements.unwrap(),
			vec![
				Statement::Option("title".to_string(), "My Beancount File".to_string()),
				Statement::Plugin("beancount.plugins.example".to_string(), None),
				Statement::Plugin(
					"beancount.plugins.example".to_string(),
					Some("arg1".to_string())
				),
				Statement::Include("file.beancount".to_string()),
				Statement::Directive(Directive::new(
					date,
					DirectiveKind::Open(
						"Assets:US:B-of-A:Checking".parse().unwrap(),
						vec!["USD".parse().unwrap(), "CAD".parse().unwrap()],
						Some("NONE".to_string().into())
					),
					HashMap::from([
						("filename".to_string(), Metadata::String("test".to_string())),
						("lineno".to_string(), Metadata::Number(Decimal::from(5))),
						(
							"value".to_string(),
							Metadata::Number(Decimal::from_str("123.456").unwrap())
						),
					]),
				)),
				Statement::Directive(Directive::new(
					date,
					DirectiveKind::Close("Assets:US:B-of-A:Checking".parse().unwrap()),
					HashMap::from([
						("filename".to_string(), Metadata::String("test".to_string())),
						("lineno".to_string(), Metadata::Number(Decimal::from(7))),
					]),
				)),
				Statement::Directive(Directive::new(
					date,
					DirectiveKind::Commodity("AAPL".parse().unwrap()),
					HashMap::from([
						("filename".to_string(), Metadata::String("test".to_string())),
						("lineno".to_string(), Metadata::Number(Decimal::from(8))),
					]),
				)),
				Statement::Directive(Directive::new(
					date,
					DirectiveKind::Pad {
						account: "Assets:US".parse().unwrap(),
						source_account: "Equity:Opening-Balances".parse().unwrap(),
					},
					HashMap::from([
						("filename".to_string(), Metadata::String("test".to_string())),
						("lineno".to_string(), Metadata::Number(Decimal::from(9))),
					]),
				)),
				Statement::Directive(Directive::new(
					date,
					DirectiveKind::Balance {
						account: "Assets:US".parse().unwrap(),
						amount: Amount::new(
							Decimal::from_str("319.020").unwrap(),
							"RGAGX".parse().unwrap()
						),
						tolerance: None,
						diff_amount: None,
					},
					HashMap::from([
						("filename".to_string(), Metadata::String("test".to_string())),
						("lineno".to_string(), Metadata::Number(Decimal::from(10))),
					]),
				)),
				Statement::Directive(Directive::new(
					date,
					DirectiveKind::Balance {
						account: "Assets:US".parse().unwrap(),
						amount: Amount::new(
							Decimal::from_str("319.020").unwrap(),
							"RGAGX".parse().unwrap()
						),
						tolerance: Some(Decimal::from_str("0.002").unwrap()),
						diff_amount: None,
					},
					HashMap::from([
						("filename".to_string(), Metadata::String("test".to_string())),
						("lineno".to_string(), Metadata::Number(Decimal::from(11))),
					]),
				)),
				Statement::Directive(Directive::new(
					date,
					DirectiveKind::Note {
						account: "Assets:US".parse().unwrap(),
						comment: "This is a note".to_string(),
						tags: HashSet::new(),
						links: HashSet::new(),
					},
					HashMap::from([
						("filename".to_string(), Metadata::String("test".to_string())),
						("lineno".to_string(), Metadata::Number(Decimal::from(12))),
					]),
				)),
				Statement::Directive(Directive::new(
					date,
					DirectiveKind::Note {
						account: "Assets:US".parse().unwrap(),
						comment: "This is a note".to_string(),
						tags: HashSet::from(["test-tag".to_string()]),
						links: HashSet::from(["test-link".to_string()]),
					},
					HashMap::from([
						("filename".to_string(), Metadata::String("test".to_string())),
						("lineno".to_string(), Metadata::Number(Decimal::from(13))),
					]),
				)),
				Statement::Directive(Directive::new(
					date,
					DirectiveKind::Event {
						kind: "location".to_string(),
						description: "Paris, France".to_string(),
					},
					HashMap::from([
						("filename".to_string(), Metadata::String("test".to_string())),
						("lineno".to_string(), Metadata::Number(Decimal::from(14))),
					]),
				)),
				Statement::Directive(Directive::new(
					date,
					DirectiveKind::Query {
						name: "france-balances".to_string(),
						query: "\n\t\t\t\tSELECT account, sum(position) WHERE ‘trip-france-2014’ in tags"
							.to_string(),
					},
					HashMap::from([
						("filename".to_string(), Metadata::String("test".to_string())),
						("lineno".to_string(), Metadata::Number(Decimal::from(15))),
					]),
				)),
				Statement::Directive(Directive::new(
					date,
					DirectiveKind::Price {
						commodity: "HOOL".parse().unwrap(),
						amount: Amount::new(
							Decimal::from_str("579.18").unwrap(),
							"USD".parse().unwrap()
						),
					},
					HashMap::from([
						("filename".to_string(), Metadata::String("test".to_string())),
						("lineno".to_string(), Metadata::Number(Decimal::from(17))),
					]),
				)),
				Statement::Directive(Directive::new(
					date,
					DirectiveKind::Document {
						account: "Liabilities:CreditCard".parse().unwrap(),
						filename: "/home/joe/stmts/apr-2014.pdf".to_string(),
						tags: HashSet::from(["test-tag".to_string()]),
						links: HashSet::from(["test-link".to_string()]),
					},
					HashMap::from([
						("filename".to_string(), Metadata::String("test".to_string())),
						("lineno".to_string(), Metadata::Number(Decimal::from(18))),
					]),
				)),
				Statement::Directive(Directive::new(
					date,
					DirectiveKind::Custom {
						kind: "budget".to_string(),
						values: vec![
							Metadata::String("...".to_string()),
							Metadata::Bool(true),
							Metadata::Amount(Amount::new(
								Decimal::from_str("4.30").unwrap(),
								"USD".parse().unwrap()
							)),
						],
					},
					HashMap::from([
						("filename".to_string(), Metadata::String("test".to_string())),
						("lineno".to_string(), Metadata::Number(Decimal::from(19))),
					]),
				)),
			],
		);
	}

	#[test]
	fn test_parser_tx() {
		let filename: Rc<str> = Rc::from("test");
		let src = r#"
			2025-01-01 txn "Cafe Mogador" "Lamb tagine with wine"
				Liabilities:CreditCard -37.45 USD
				Expenses:Restaurants
		"#;

		let date = NaiveDate::from_str("2025-01-01").unwrap();

		let (statements, _errors) = parse_str(filename.clone(), src);

		assert_eq!(
			statements.unwrap(),
			vec![Statement::Directive(Directive::new(
				date,
				DirectiveKind::Custom {
					kind: "budget".to_string(),
					values: vec![
						Metadata::String("...".to_string()),
						Metadata::Bool(true),
						Metadata::Amount(Amount::new(
							Decimal::from_str("4.30").unwrap(),
							"USD".parse().unwrap()
						)),
					],
				},
				HashMap::from([
					("filename".to_string(), Metadata::String("test".to_string())),
					("lineno".to_string(), Metadata::Number(Decimal::from(1))),
				]),
			)),],
		);
	}
}
