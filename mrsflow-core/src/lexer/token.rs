//! Token types for the M lexer.

/// Half-open byte range into the source string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Token classes recognised by the lexer.
///
/// Slice 2a adds the remaining operators (comparison, `=>`, `..`, `...`, `??`,
/// `@`, `!`, `?`) and slice 2b adds hex numbers and exponent parts.
/// Verbatim literals (`#!"..."`), quoted identifiers (`#"..."`), the
/// `#date`/`#datetime`/etc. keywords, character escape sequences (`#(cr,lf)`),
/// and full Unicode identifier classes are deferred to slice 2c+.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals — the raw lexeme is preserved; numeric parsing happens at
    // evaluation time so we don't lose precision through the lexer.
    Number(String),
    Text(String),
    True,
    False,
    Null,

    /// Regular identifier, possibly dotted (`Table.SelectRows` is one token).
    Identifier(String),
    /// Quoted identifier, e.g. `#"with spaces"` — value carries the raw chars
    /// between the quotes, with `""` un-escaped to a single `"`.
    QuotedIdentifier(String),
    /// Verbatim literal, e.g. `#!"unparseable"` — produces an error value at
    /// runtime per spec; lexer just preserves the raw chars.
    VerbatimLiteral(String),

    // Keywords.
    Let,
    In,
    If,
    Then,
    Else,
    And,
    Or,
    Not,
    Each,
    Try,
    Otherwise,
    Error,
    As,
    Is,
    Type,
    Meta,
    Section,
    Shared,

    // # keywords — reserved words that introduce intrinsic-form expressions.
    HashBinary,
    HashDate,
    HashDatetime,
    HashDatetimezone,
    HashDuration,
    HashInfinity,
    HashNan,
    HashSections,
    HashShared,
    HashTable,
    HashTime,

    // Operators and punctuators.
    Equals,           // =
    Plus,             // +
    Minus,            // -
    Star,             // *
    Slash,            // /
    Ampersand,        // &
    LeftParen,        // (
    RightParen,       // )
    LeftBracket,      // [
    RightBracket,     // ]
    LeftBrace,        // {
    RightBrace,       // }
    Comma,            // ,
    Semicolon,        // ;
    LessThan,         // <
    LessEquals,       // <=
    GreaterThan,      // >
    GreaterEquals,    // >=
    NotEquals,        // <>
    FatArrow,         // =>
    DotDot,           // ..
    Ellipsis,         // ...
    Question,         // ?
    QuestionQuestion, // ??
    At,               // @
    Bang,             // !
}
