//! Dump tokens for a source file in a normalised format.
//!
//! Used by `tools/grammar-fuzz/diff.sh` to differential-test the Rust lexer
//! against the Prolog DCG. Output format must match `print_token/1` in
//! `tools/grammar-fuzz/lexical.pl` exactly.
//!
//! Usage: lex_dump <path>

use mrsflow_core::lexer::{tokenize, TokenKind};
use std::env;
use std::fs;
use std::process;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: lex_dump <path>");
        process::exit(64);
    });
    let src = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("read {}: {}", path, e);
        process::exit(66);
    });
    match tokenize(&src) {
        Ok(toks) => {
            for t in toks {
                print_token(&t.kind);
            }
        }
        Err(e) => {
            eprintln!("LEX ERROR: {:?}", e);
            process::exit(2);
        }
    }
}

fn print_token(k: &TokenKind) {
    match k {
        TokenKind::Number(s) => println!("Number {}", s),
        TokenKind::Text(s) => println!("Text {}", s),
        TokenKind::Identifier(s) => println!("Identifier {}", s),
        TokenKind::QuotedIdentifier(s) => println!("QuotedIdentifier {}", s),
        TokenKind::VerbatimLiteral(s) => println!("VerbatimLiteral {}", s),
        TokenKind::Let => println!("Keyword let"),
        TokenKind::In => println!("Keyword in"),
        TokenKind::If => println!("Keyword if"),
        TokenKind::Then => println!("Keyword then"),
        TokenKind::Else => println!("Keyword else"),
        TokenKind::True => println!("Keyword true"),
        TokenKind::False => println!("Keyword false"),
        TokenKind::Null => println!("Keyword null"),
        TokenKind::And => println!("Keyword and"),
        TokenKind::Or => println!("Keyword or"),
        TokenKind::Not => println!("Keyword not"),
        TokenKind::Each => println!("Keyword each"),
        TokenKind::Try => println!("Keyword try"),
        TokenKind::Otherwise => println!("Keyword otherwise"),
        TokenKind::Error => println!("Keyword error"),
        TokenKind::As => println!("Keyword as"),
        TokenKind::Is => println!("Keyword is"),
        TokenKind::Type => println!("Keyword type"),
        TokenKind::Meta => println!("Keyword meta"),
        TokenKind::Section => println!("Keyword section"),
        TokenKind::Shared => println!("Keyword shared"),
        TokenKind::HashBinary => println!("Keyword #binary"),
        TokenKind::HashDate => println!("Keyword #date"),
        TokenKind::HashDatetime => println!("Keyword #datetime"),
        TokenKind::HashDatetimezone => println!("Keyword #datetimezone"),
        TokenKind::HashDuration => println!("Keyword #duration"),
        TokenKind::HashInfinity => println!("Keyword #infinity"),
        TokenKind::HashNan => println!("Keyword #nan"),
        TokenKind::HashSections => println!("Keyword #sections"),
        TokenKind::HashShared => println!("Keyword #shared"),
        TokenKind::HashTable => println!("Keyword #table"),
        TokenKind::HashTime => println!("Keyword #time"),
        TokenKind::Equals => println!("Op equals"),
        TokenKind::Plus => println!("Op plus"),
        TokenKind::Minus => println!("Op minus"),
        TokenKind::Star => println!("Op star"),
        TokenKind::Slash => println!("Op slash"),
        TokenKind::Ampersand => println!("Op ampersand"),
        TokenKind::LeftParen => println!("Op lparen"),
        TokenKind::RightParen => println!("Op rparen"),
        TokenKind::LeftBracket => println!("Op lbracket"),
        TokenKind::RightBracket => println!("Op rbracket"),
        TokenKind::LeftBrace => println!("Op lbrace"),
        TokenKind::RightBrace => println!("Op rbrace"),
        TokenKind::Comma => println!("Op comma"),
        TokenKind::Semicolon => println!("Op semicolon"),
        TokenKind::LessThan => println!("Op lt"),
        TokenKind::LessEquals => println!("Op le"),
        TokenKind::GreaterThan => println!("Op gt"),
        TokenKind::GreaterEquals => println!("Op ge"),
        TokenKind::NotEquals => println!("Op ne"),
        TokenKind::FatArrow => println!("Op fat_arrow"),
        TokenKind::DotDot => println!("Op dot_dot"),
        TokenKind::Ellipsis => println!("Op ellipsis"),
        TokenKind::Question => println!("Op question"),
        TokenKind::QuestionQuestion => println!("Op null_coalesce"),
        TokenKind::At => println!("Op at"),
        TokenKind::Bang => println!("Op bang"),
    }
}
