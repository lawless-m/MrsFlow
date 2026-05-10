//! Lexer for the M language.
//!
//! Reference: Microsoft Power Query M language specification, lexical structure.
//! Spec is the source of truth; this lexer is one reading of it. The Prolog DCG
//! companion in `tools/grammar-fuzz/` is an independent second reading.
//!
//! Slice 1 scope is documented on `TokenKind`. Anything outside that scope
//! produces a `LexError::UnexpectedChar` rather than silently accepting it.

mod token;

pub use token::{Span, Token, TokenKind};

use std::iter::Peekable;
use std::str::CharIndices;

#[derive(Debug, Clone, PartialEq)]
pub enum LexError {
    UnexpectedChar { pos: usize, ch: char },
    UnterminatedText { start: usize },
    UnterminatedComment { start: usize },
    /// A decimal point not followed by at least one digit (`1.` is invalid per spec).
    InvalidNumber { span: Span },
    /// A `#(...)` character-escape sequence with malformed contents
    /// (unrecognised control name, wrong-length hex, missing `)` or `,`, etc.).
    InvalidEscape { pos: usize },
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).run()
}

struct Lexer<'src> {
    source: &'src str,
    chars: Peekable<CharIndices<'src>>,
}

impl<'src> Lexer<'src> {
    fn new(source: &'src str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
        }
    }

    fn run(mut self) -> Result<Vec<Token>, LexError> {
        let mut out = Vec::new();
        loop {
            self.skip_trivia()?;
            let Some(&(start, ch)) = self.chars.peek() else {
                return Ok(out);
            };
            let token = self.scan_token(start, ch)?;
            out.push(token);
        }
    }

    fn skip_trivia(&mut self) -> Result<(), LexError> {
        loop {
            match self.chars.peek().copied() {
                Some((_, ch)) if ch.is_whitespace() => {
                    self.chars.next();
                }
                Some((start, '/')) => {
                    let mut lookahead = self.chars.clone();
                    lookahead.next();
                    match lookahead.peek().copied() {
                        Some((_, '/')) => self.skip_line_comment(),
                        Some((_, '*')) => self.skip_delimited_comment(start)?,
                        _ => return Ok(()),
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn skip_line_comment(&mut self) {
        // Caller already saw `//` via lookahead; consume the slashes and run to newline.
        self.chars.next();
        self.chars.next();
        while let Some(&(_, ch)) = self.chars.peek() {
            if matches!(ch, '\n' | '\r' | '\u{0085}' | '\u{2028}' | '\u{2029}') {
                break;
            }
            self.chars.next();
        }
    }

    fn skip_delimited_comment(&mut self, start: usize) -> Result<(), LexError> {
        self.chars.next(); // consume '/'
        self.chars.next(); // consume '*'
        loop {
            match self.chars.next() {
                None => return Err(LexError::UnterminatedComment { start }),
                Some((_, '*')) => {
                    if let Some(&(_, '/')) = self.chars.peek() {
                        self.chars.next();
                        return Ok(());
                    }
                }
                Some(_) => {}
            }
        }
    }

    fn scan_token(&mut self, start: usize, ch: char) -> Result<Token, LexError> {
        match ch {
            '"' => self.scan_text(start),
            '#' => self.scan_hash(start),
            '0'..='9' => self.scan_number(start),
            c if is_identifier_start(c) => Ok(self.scan_identifier_or_keyword(start)),
            _ => self.scan_operator(start, ch),
        }
    }

    /// Dispatch on what follows `#`:
    ///   #"..."  → quoted identifier
    ///   #!"..." → verbatim literal
    ///   #word   → one of the # keywords (anything else is a lex error)
    fn scan_hash(&mut self, start: usize) -> Result<Token, LexError> {
        self.chars.next(); // consume '#'
        match self.peek_offset(0) {
            Some('"') => self.scan_quoted_identifier(start),
            Some('!') if self.peek_offset(1) == Some('"') => {
                self.chars.next(); // consume '!'
                self.scan_verbatim_literal(start)
            }
            Some(c) if is_identifier_start(c) => self.scan_hash_keyword(start),
            _ => Err(LexError::UnexpectedChar { pos: start, ch: '#' }),
        }
    }

    fn scan_quoted_identifier(&mut self, start: usize) -> Result<Token, LexError> {
        // Cursor is on the opening `"`. Body parsing is identical to text literal.
        let value = self.scan_quoted_body(start)?;
        let end = self.cursor_pos();
        Ok(Token {
            kind: TokenKind::QuotedIdentifier(value),
            span: Span::new(start, end),
        })
    }

    fn scan_verbatim_literal(&mut self, start: usize) -> Result<Token, LexError> {
        let value = self.scan_quoted_body(start)?;
        let end = self.cursor_pos();
        Ok(Token {
            kind: TokenKind::VerbatimLiteral(value),
            span: Span::new(start, end),
        })
    }

    /// Consume `"`...`"`, returning the unescaped value. Doubled `""` is a
    /// single quote in the value, and `#(...)` is a character escape sequence.
    /// Used by text literals, quoted identifiers, and verbatim literals — they
    /// all share the same body shape per the spec.
    fn scan_quoted_body(&mut self, start: usize) -> Result<String, LexError> {
        self.chars.next(); // opening "
        let mut value = String::new();
        loop {
            match self.chars.peek().copied() {
                None => return Err(LexError::UnterminatedText { start }),
                Some((_, '"')) => {
                    self.chars.next();
                    if let Some(&(_, '"')) = self.chars.peek() {
                        self.chars.next();
                        value.push('"');
                    } else {
                        return Ok(value);
                    }
                }
                // `#(` introduces a character-escape sequence. A `#` followed
                // by anything else is just a literal `#`.
                Some((esc_pos, '#')) if self.peek_offset(1) == Some('(') => {
                    self.chars.next(); // #
                    self.chars.next(); // (
                    self.consume_escape_list(&mut value, esc_pos)?;
                }
                Some((_, _)) => {
                    let (_, c) = self.chars.next().unwrap();
                    value.push(c);
                }
            }
        }
    }

    /// Parse the body of a `#(...)` escape: one or more single-escapes
    /// separated by commas, terminated by `)`.
    fn consume_escape_list(&mut self, out: &mut String, pos: usize) -> Result<(), LexError> {
        loop {
            self.consume_single_escape(out, pos)?;
            match self.chars.peek().copied() {
                Some((_, ',')) => {
                    self.chars.next();
                }
                Some((_, ')')) => {
                    self.chars.next();
                    return Ok(());
                }
                _ => return Err(LexError::InvalidEscape { pos }),
            }
        }
    }

    fn consume_single_escape(&mut self, out: &mut String, pos: usize) -> Result<(), LexError> {
        // escape-escape: `#` produces a literal `#`.
        if self.peek_offset(0) == Some('#') {
            self.chars.next();
            out.push('#');
            return Ok(());
        }
        // control-character-escape: cr | lf | tab. Try these before hex
        // because `cr` starts with `c` which is also a hex digit but only
        // 1 char long → would fail hex-length check anyway.
        if let Some(name_len) = self.match_control_escape() {
            let ch = match self.snapshot_chars(name_len) {
                s if s == "cr" => '\r',
                s if s == "lf" => '\n',
                s if s == "tab" => '\t',
                _ => unreachable!(),
            };
            for _ in 0..name_len {
                self.chars.next();
            }
            out.push(ch);
            return Ok(());
        }
        // unicode-escape: 4 (short) or 8 (long) hex digits, exact length.
        let mut hex = String::new();
        let mut peek = self.chars.clone();
        while hex.len() < 8 {
            match peek.peek().copied() {
                Some((_, c)) if c.is_ascii_hexdigit() => {
                    hex.push(c);
                    peek.next();
                }
                _ => break,
            }
        }
        if hex.len() != 4 && hex.len() != 8 {
            return Err(LexError::InvalidEscape { pos });
        }
        let cp = u32::from_str_radix(&hex, 16)
            .map_err(|_| LexError::InvalidEscape { pos })?;
        let ch = char::from_u32(cp).ok_or(LexError::InvalidEscape { pos })?;
        for _ in 0..hex.len() {
            self.chars.next();
        }
        out.push(ch);
        Ok(())
    }

    fn match_control_escape(&self) -> Option<usize> {
        let s2 = self.snapshot_chars(2);
        if s2 == "cr" || s2 == "lf" {
            return Some(2);
        }
        let s3 = self.snapshot_chars(3);
        if s3 == "tab" {
            return Some(3);
        }
        None
    }

    fn snapshot_chars(&self, n: usize) -> String {
        let mut it = self.chars.clone();
        let mut s = String::new();
        for _ in 0..n {
            match it.next() {
                Some((_, c)) => s.push(c),
                None => break,
            }
        }
        s
    }

    fn scan_hash_keyword(&mut self, start: usize) -> Result<Token, LexError> {
        let name_start = self.cursor_pos();
        // Consume identifier-shape after `#`.
        while let Some(&(_, c)) = self.chars.peek() {
            if is_identifier_part(c) {
                self.chars.next();
            } else {
                break;
            }
        }
        let end = self.cursor_pos();
        let name = &self.source[name_start..end];
        let kind = match name {
            "binary" => TokenKind::HashBinary,
            "date" => TokenKind::HashDate,
            "datetime" => TokenKind::HashDatetime,
            "datetimezone" => TokenKind::HashDatetimezone,
            "duration" => TokenKind::HashDuration,
            "infinity" => TokenKind::HashInfinity,
            "nan" => TokenKind::HashNan,
            "sections" => TokenKind::HashSections,
            "shared" => TokenKind::HashShared,
            "table" => TokenKind::HashTable,
            "time" => TokenKind::HashTime,
            _ => return Err(LexError::UnexpectedChar { pos: start, ch: '#' }),
        };
        Ok(Token {
            kind,
            span: Span::new(start, end),
        })
    }

    fn cursor_pos(&self) -> usize {
        let mut it = self.chars.clone();
        match it.peek() {
            Some(&(p, _)) => p,
            None => self.source.len(),
        }
    }

    fn scan_text(&mut self, start: usize) -> Result<Token, LexError> {
        let value = self.scan_quoted_body(start)?;
        Ok(Token {
            kind: TokenKind::Text(value),
            span: Span::new(start, self.cursor_pos()),
        })
    }

    fn scan_number(&mut self, start: usize) -> Result<Token, LexError> {
        // Hex prefix: 0x or 0X followed by at least one hex digit.
        if self.peek_offset(0) == Some('0')
            && matches!(self.peek_offset(1), Some('x') | Some('X'))
        {
            return self.scan_hex_number(start);
        }

        let mut end = start;
        end = self.consume_decimal_digits(end);

        // Optional fractional part: `.` followed by at least one digit.
        // A `.` followed by another `.` is the `..` operator (only legal in
        // list-item ranges) — leave it for scan_operator, the integer stands.
        if let Some(&(dot_pos, '.')) = self.chars.peek() {
            match self.peek_offset(1) {
                Some(d) if d.is_ascii_digit() => {
                    self.chars.next(); // consume '.'
                    end = dot_pos + 1;
                    end = self.consume_decimal_digits(end);
                }
                Some('.') => {
                    // `..` follows — number is just the integer; don't consume the dot.
                }
                _ => {
                    return Err(LexError::InvalidNumber {
                        span: Span::new(start, dot_pos + 1),
                    });
                }
            }
        }

        // Optional exponent: e|E, optional sign, mandatory digits.
        if let Some(&(e_pos, e_ch)) = self.chars.peek() {
            if e_ch == 'e' || e_ch == 'E' {
                let after_sign = match self.peek_offset(1) {
                    Some('+') | Some('-') => Some(self.peek_offset(2)),
                    other => Some(other),
                };
                match after_sign.flatten() {
                    Some(d) if d.is_ascii_digit() => {
                        self.chars.next(); // e/E
                        end = e_pos + 1;
                        if matches!(self.peek_offset(0), Some('+') | Some('-')) {
                            let (pos, c) = self.chars.next().unwrap();
                            end = pos + c.len_utf8();
                        }
                        end = self.consume_decimal_digits(end);
                    }
                    _ => {
                        return Err(LexError::InvalidNumber {
                            span: Span::new(start, e_pos + 1),
                        });
                    }
                }
            }
        }

        let lexeme = self.source[start..end].to_string();
        Ok(Token {
            kind: TokenKind::Number(lexeme),
            span: Span::new(start, end),
        })
    }

    fn scan_hex_number(&mut self, start: usize) -> Result<Token, LexError> {
        self.chars.next(); // '0'
        self.chars.next(); // 'x' or 'X'
        let mut end = start + 2;
        let mut any = false;
        while let Some(&(pos, c)) = self.chars.peek() {
            if c.is_ascii_hexdigit() {
                end = pos + c.len_utf8();
                self.chars.next();
                any = true;
            } else {
                break;
            }
        }
        if !any {
            return Err(LexError::InvalidNumber {
                span: Span::new(start, end),
            });
        }
        let lexeme = self.source[start..end].to_string();
        Ok(Token {
            kind: TokenKind::Number(lexeme),
            span: Span::new(start, end),
        })
    }

    fn consume_decimal_digits(&mut self, mut end: usize) -> usize {
        while let Some(&(pos, c)) = self.chars.peek() {
            if c.is_ascii_digit() {
                end = pos + c.len_utf8();
                self.chars.next();
            } else {
                break;
            }
        }
        end
    }

    fn scan_identifier_or_keyword(&mut self, start: usize) -> Token {
        let mut end = start;
        // First segment.
        while let Some(&(pos, c)) = self.chars.peek() {
            if is_identifier_part(c) {
                end = pos + c.len_utf8();
                self.chars.next();
            } else {
                break;
            }
        }
        // Dotted continuations: `.X` where X is identifier-start.
        loop {
            let mut lookahead = self.chars.clone();
            let Some(&(dot_pos, '.')) = lookahead.peek() else {
                break;
            };
            lookahead.next();
            let Some(&(_, next)) = lookahead.peek() else {
                break;
            };
            if !is_identifier_start(next) {
                break;
            }
            self.chars.next(); // '.'
            end = dot_pos + 1;
            while let Some(&(pos, c)) = self.chars.peek() {
                if is_identifier_part(c) {
                    end = pos + c.len_utf8();
                    self.chars.next();
                } else {
                    break;
                }
            }
        }
        let lexeme = &self.source[start..end];
        let kind = match lexeme {
            "let" => TokenKind::Let,
            "in" => TokenKind::In,
            "if" => TokenKind::If,
            "then" => TokenKind::Then,
            "else" => TokenKind::Else,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "each" => TokenKind::Each,
            "try" => TokenKind::Try,
            "otherwise" => TokenKind::Otherwise,
            "error" => TokenKind::Error,
            "as" => TokenKind::As,
            "is" => TokenKind::Is,
            "type" => TokenKind::Type,
            "meta" => TokenKind::Meta,
            "section" => TokenKind::Section,
            "shared" => TokenKind::Shared,
            other => TokenKind::Identifier(other.to_string()),
        };
        Token {
            kind,
            span: Span::new(start, end),
        }
    }

    fn scan_operator(&mut self, start: usize, ch: char) -> Result<Token, LexError> {
        // Single-char operators with no possible multi-char extension.
        let single = |k: TokenKind| -> Option<(TokenKind, usize)> { Some((k, 1)) };
        let (kind, len) = match ch {
            '+' => single(TokenKind::Plus).unwrap(),
            '-' => single(TokenKind::Minus).unwrap(),
            '*' => single(TokenKind::Star).unwrap(),
            '/' => single(TokenKind::Slash).unwrap(),
            '&' => single(TokenKind::Ampersand).unwrap(),
            '(' => single(TokenKind::LeftParen).unwrap(),
            ')' => single(TokenKind::RightParen).unwrap(),
            '[' => single(TokenKind::LeftBracket).unwrap(),
            ']' => single(TokenKind::RightBracket).unwrap(),
            '{' => single(TokenKind::LeftBrace).unwrap(),
            '}' => single(TokenKind::RightBrace).unwrap(),
            ',' => single(TokenKind::Comma).unwrap(),
            ';' => single(TokenKind::Semicolon).unwrap(),
            '@' => single(TokenKind::At).unwrap(),
            '!' => single(TokenKind::Bang).unwrap(),
            '=' => match self.peek_offset(1) {
                Some('>') => (TokenKind::FatArrow, 2),
                _ => (TokenKind::Equals, 1),
            },
            '<' => match self.peek_offset(1) {
                Some('=') => (TokenKind::LessEquals, 2),
                Some('>') => (TokenKind::NotEquals, 2),
                _ => (TokenKind::LessThan, 1),
            },
            '>' => match self.peek_offset(1) {
                Some('=') => (TokenKind::GreaterEquals, 2),
                _ => (TokenKind::GreaterThan, 1),
            },
            '?' => match self.peek_offset(1) {
                Some('?') => (TokenKind::QuestionQuestion, 2),
                _ => (TokenKind::Question, 1),
            },
            // `.` outside of a number or identifier must be part of `..` or `...`.
            '.' => match (self.peek_offset(1), self.peek_offset(2)) {
                (Some('.'), Some('.')) => (TokenKind::Ellipsis, 3),
                (Some('.'), _) => (TokenKind::DotDot, 2),
                _ => return Err(LexError::UnexpectedChar { pos: start, ch }),
            },
            _ => return Err(LexError::UnexpectedChar { pos: start, ch }),
        };
        for _ in 0..len {
            self.chars.next();
        }
        Ok(Token {
            kind,
            span: Span::new(start, start + len),
        })
    }

    /// Look at the character `offset` positions ahead of the current cursor,
    /// without consuming. `offset == 0` is the current peek.
    fn peek_offset(&self, offset: usize) -> Option<char> {
        let mut it = self.chars.clone();
        for _ in 0..offset {
            it.next();
        }
        it.peek().map(|&(_, c)| c)
    }
}

fn is_identifier_start(c: char) -> bool {
    use unicode_general_category::GeneralCategory as G;
    if c == '_' {
        return true;
    }
    matches!(
        unicode_general_category::get_general_category(c),
        G::UppercaseLetter   // Lu
        | G::LowercaseLetter // Ll
        | G::TitlecaseLetter // Lt
        | G::ModifierLetter  // Lm
        | G::OtherLetter     // Lo
        | G::LetterNumber    // Nl
    )
}

fn is_identifier_part(c: char) -> bool {
    use unicode_general_category::GeneralCategory as G;
    if is_identifier_start(c) {
        return true;
    }
    matches!(
        unicode_general_category::get_general_category(c),
        G::DecimalNumber          // Nd
        | G::ConnectorPunctuation // Pc
        | G::NonspacingMark       // Mn
        | G::SpacingMark          // Mc
        | G::Format               // Cf
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        tokenize(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn empty_source_produces_no_tokens() {
        assert!(tokenize("").unwrap().is_empty());
        assert!(tokenize("   \n\t  ").unwrap().is_empty());
    }

    #[test]
    fn line_comment_skipped() {
        assert!(tokenize("// just a comment").unwrap().is_empty());
        assert_eq!(kinds("// hi\n42"), vec![TokenKind::Number("42".into())]);
    }

    #[test]
    fn delimited_comment_skipped() {
        assert!(tokenize("/* hi */").unwrap().is_empty());
        assert_eq!(
            kinds("/* a\nb */ 42"),
            vec![TokenKind::Number("42".into())]
        );
    }

    #[test]
    fn unterminated_comment_errors() {
        assert!(matches!(
            tokenize("/* never closed"),
            Err(LexError::UnterminatedComment { start: 0 })
        ));
    }

    #[test]
    fn integer_literal() {
        assert_eq!(kinds("0"), vec![TokenKind::Number("0".into())]);
        assert_eq!(kinds("12345"), vec![TokenKind::Number("12345".into())]);
    }

    #[test]
    fn decimal_literal() {
        assert_eq!(kinds("3.14"), vec![TokenKind::Number("3.14".into())]);
    }

    #[test]
    fn trailing_dot_is_invalid_number() {
        assert!(matches!(
            tokenize("1."),
            Err(LexError::InvalidNumber { .. })
        ));
    }

    #[test]
    fn text_literal_simple() {
        assert_eq!(
            kinds(r#""hello""#),
            vec![TokenKind::Text("hello".into())]
        );
    }

    #[test]
    fn text_literal_with_doubled_quote() {
        assert_eq!(
            kinds(r#""he said ""hi""""#),
            vec![TokenKind::Text(r#"he said "hi""#.into())]
        );
    }

    #[test]
    fn unterminated_text_errors() {
        assert!(matches!(
            tokenize(r#""nope"#),
            Err(LexError::UnterminatedText { start: 0 })
        ));
    }

    #[test]
    fn keywords_recognised() {
        assert_eq!(
            kinds("let in if then else true false null and or not"),
            vec![
                TokenKind::Let,
                TokenKind::In,
                TokenKind::If,
                TokenKind::Then,
                TokenKind::Else,
                TokenKind::True,
                TokenKind::False,
                TokenKind::Null,
                TokenKind::And,
                TokenKind::Or,
                TokenKind::Not,
            ]
        );
    }

    #[test]
    fn identifier_simple() {
        assert_eq!(
            kinds("foo Bar _baz qux123"),
            vec![
                TokenKind::Identifier("foo".into()),
                TokenKind::Identifier("Bar".into()),
                TokenKind::Identifier("_baz".into()),
                TokenKind::Identifier("qux123".into()),
            ]
        );
    }

    #[test]
    fn identifier_dotted_is_one_token() {
        assert_eq!(
            kinds("Table.SelectRows"),
            vec![TokenKind::Identifier("Table.SelectRows".into())]
        );
        assert_eq!(
            kinds("A.B.C"),
            vec![TokenKind::Identifier("A.B.C".into())]
        );
    }

    #[test]
    fn operators() {
        assert_eq!(
            kinds("= + - * / & ( ) [ ] { } , ;"),
            vec![
                TokenKind::Equals,
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Ampersand,
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::LeftBracket,
                TokenKind::RightBracket,
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::Comma,
                TokenKind::Semicolon,
            ]
        );
    }

    #[test]
    fn realistic_let_in_expression() {
        let src = "let x = 1 + 2, y = x * 3 in y";
        assert_eq!(
            kinds(src),
            vec![
                TokenKind::Let,
                TokenKind::Identifier("x".into()),
                TokenKind::Equals,
                TokenKind::Number("1".into()),
                TokenKind::Plus,
                TokenKind::Number("2".into()),
                TokenKind::Comma,
                TokenKind::Identifier("y".into()),
                TokenKind::Equals,
                TokenKind::Identifier("x".into()),
                TokenKind::Star,
                TokenKind::Number("3".into()),
                TokenKind::In,
                TokenKind::Identifier("y".into()),
            ]
        );
    }

    #[test]
    fn spans_are_byte_accurate() {
        let toks = tokenize("foo + 42").unwrap();
        assert_eq!(toks[0].span, Span::new(0, 3));   // foo
        assert_eq!(toks[1].span, Span::new(4, 5));   // +
        assert_eq!(toks[2].span, Span::new(6, 8));   // 42
    }

    #[test]
    fn unexpected_char_reported() {
        assert!(matches!(
            tokenize("$"),
            Err(LexError::UnexpectedChar { pos: 0, ch: '$' })
        ));
    }

    #[test]
    fn comparison_operators() {
        assert_eq!(
            kinds("< <= > >= <>"),
            vec![
                TokenKind::LessThan,
                TokenKind::LessEquals,
                TokenKind::GreaterThan,
                TokenKind::GreaterEquals,
                TokenKind::NotEquals,
            ]
        );
    }

    #[test]
    fn fat_arrow() {
        assert_eq!(
            kinds("(x) => x + 1"),
            vec![
                TokenKind::LeftParen,
                TokenKind::Identifier("x".into()),
                TokenKind::RightParen,
                TokenKind::FatArrow,
                TokenKind::Identifier("x".into()),
                TokenKind::Plus,
                TokenKind::Number("1".into()),
            ]
        );
    }

    #[test]
    fn dot_operators() {
        assert_eq!(
            kinds(".. ..."),
            vec![TokenKind::DotDot, TokenKind::Ellipsis]
        );
    }

    #[test]
    fn lone_dot_is_invalid() {
        assert!(matches!(
            tokenize(". x"),
            Err(LexError::UnexpectedChar { pos: 0, ch: '.' })
        ));
    }

    #[test]
    fn question_and_null_coalesce() {
        assert_eq!(
            kinds("? ??"),
            vec![TokenKind::Question, TokenKind::QuestionQuestion]
        );
    }

    #[test]
    fn at_and_bang() {
        assert_eq!(kinds("@ !"), vec![TokenKind::At, TokenKind::Bang]);
    }

    #[test]
    fn hex_number() {
        assert_eq!(kinds("0xff"), vec![TokenKind::Number("0xff".into())]);
        assert_eq!(kinds("0X1A"), vec![TokenKind::Number("0X1A".into())]);
        assert_eq!(kinds("0xDeadBeef"), vec![TokenKind::Number("0xDeadBeef".into())]);
    }

    #[test]
    fn hex_without_digits_is_invalid() {
        assert!(matches!(
            tokenize("0x"),
            Err(LexError::InvalidNumber { .. })
        ));
    }

    #[test]
    fn exponent_simple() {
        assert_eq!(kinds("1e3"), vec![TokenKind::Number("1e3".into())]);
        assert_eq!(kinds("2E10"), vec![TokenKind::Number("2E10".into())]);
    }

    #[test]
    fn exponent_with_sign() {
        assert_eq!(kinds("1e+3"), vec![TokenKind::Number("1e+3".into())]);
        assert_eq!(kinds("1.5e-10"), vec![TokenKind::Number("1.5e-10".into())]);
    }

    #[test]
    fn exponent_without_digits_is_invalid() {
        assert!(matches!(
            tokenize("1e"),
            Err(LexError::InvalidNumber { .. })
        ));
        assert!(matches!(
            tokenize("1e+"),
            Err(LexError::InvalidNumber { .. })
        ));
    }

    #[test]
    fn number_followed_by_identifier_is_two_tokens() {
        // `1e` is invalid as a number (mandatory digits after e), but the lexer
        // shouldn't speculatively consume `e` here — that path errors. By
        // contrast `1foo` should be `1` then `foo`.
        assert_eq!(
            kinds("1 foo"),
            vec![TokenKind::Number("1".into()), TokenKind::Identifier("foo".into())]
        );
    }

    #[test]
    fn text_escape_control_names() {
        assert_eq!(
            kinds(r##""hi#(cr)""##),
            vec![TokenKind::Text("hi\r".into())]
        );
        assert_eq!(
            kinds(r##""#(cr,lf)""##),
            vec![TokenKind::Text("\r\n".into())]
        );
        assert_eq!(
            kinds(r##""#(tab)X""##),
            vec![TokenKind::Text("\tX".into())]
        );
    }

    #[test]
    fn text_escape_short_unicode() {
        // U+0041 = 'A'
        assert_eq!(
            kinds(r##""#(0041)""##),
            vec![TokenKind::Text("A".into())]
        );
    }

    #[test]
    fn text_escape_long_unicode() {
        // U+0000004D = 'M'
        assert_eq!(
            kinds(r##""#(0000004D)""##),
            vec![TokenKind::Text("M".into())]
        );
    }

    #[test]
    fn text_escape_escape_hash() {
        // #(#) produces a literal #.
        assert_eq!(
            kinds(r##""#(#)(""##),
            vec![TokenKind::Text("#(".into())]
        );
    }

    #[test]
    fn text_literal_hash_without_paren_is_literal() {
        // # not followed by ( is just a literal #.
        assert_eq!(
            kinds(r##""a#b""##),
            vec![TokenKind::Text("a#b".into())]
        );
    }

    #[test]
    fn text_escape_invalid_hex_length() {
        assert!(matches!(
            tokenize(r##""#(123)""##),
            Err(LexError::InvalidEscape { .. })
        ));
    }

    #[test]
    fn text_escape_unknown_name() {
        assert!(matches!(
            tokenize(r##""#(crlf)""##),
            Err(LexError::InvalidEscape { .. })
        ));
    }

    #[test]
    fn quoted_identifier_with_escape() {
        // Quoted identifiers share the same body grammar — escapes apply.
        assert_eq!(
            kinds(r##"#"x#(cr)y""##),
            vec![TokenKind::QuotedIdentifier("x\ry".into())]
        );
    }

    #[test]
    fn unicode_identifier_letters() {
        // Lo (Other Letter), Ll (Lowercase) — all valid as identifier chars per spec.
        assert_eq!(
            kinds("中文"),
            vec![TokenKind::Identifier("中文".into())]
        );
        assert_eq!(
            kinds("café αβγ"),
            vec![
                TokenKind::Identifier("café".into()),
                TokenKind::Identifier("αβγ".into()),
            ]
        );
    }

    #[test]
    fn unicode_letter_number_is_ident_start() {
        // Ⅷ is Nl (Letter Number, U+2167) — valid identifier start per spec.
        assert_eq!(
            kinds("Ⅷ"),
            vec![TokenKind::Identifier("Ⅷ".into())]
        );
    }

    #[test]
    fn unicode_decimal_digit_in_ident_part() {
        // Arabic-Indic digit one (U+0661) is Nd — valid in part position only.
        assert_eq!(
            kinds("name١"),
            vec![TokenKind::Identifier("name١".into())]
        );
    }

    #[test]
    fn quoted_identifier() {
        assert_eq!(
            kinds(r##"#"with spaces""##),
            vec![TokenKind::QuotedIdentifier("with spaces".into())]
        );
    }

    #[test]
    fn quoted_identifier_with_doubled_quote() {
        assert_eq!(
            kinds(r##"#"a""b""##),
            vec![TokenKind::QuotedIdentifier(r#"a"b"#.into())]
        );
    }

    #[test]
    fn verbatim_literal() {
        assert_eq!(
            kinds(r##"#!"unparseable code""##),
            vec![TokenKind::VerbatimLiteral("unparseable code".into())]
        );
    }

    #[test]
    fn hash_keywords() {
        assert_eq!(
            kinds("#date #datetime #datetimezone #duration #binary #table #time #sections #shared #infinity #nan"),
            vec![
                TokenKind::HashDate,
                TokenKind::HashDatetime,
                TokenKind::HashDatetimezone,
                TokenKind::HashDuration,
                TokenKind::HashBinary,
                TokenKind::HashTable,
                TokenKind::HashTime,
                TokenKind::HashSections,
                TokenKind::HashShared,
                TokenKind::HashInfinity,
                TokenKind::HashNan,
            ]
        );
    }

    #[test]
    fn unknown_hash_keyword_is_error() {
        assert!(matches!(
            tokenize("#bogus"),
            Err(LexError::UnexpectedChar { ch: '#', .. })
        ));
    }

    #[test]
    fn lone_hash_is_error() {
        assert!(matches!(
            tokenize("# "),
            Err(LexError::UnexpectedChar { ch: '#', .. })
        ));
    }

    #[test]
    fn longest_match_for_overlapping_operators() {
        // `<=` and `<>` must beat `<`; `=>` must beat `=`; `..` must beat `.`.
        assert_eq!(
            kinds("a<=b a<>b a=>b a..b"),
            vec![
                TokenKind::Identifier("a".into()),
                TokenKind::LessEquals,
                TokenKind::Identifier("b".into()),
                TokenKind::Identifier("a".into()),
                TokenKind::NotEquals,
                TokenKind::Identifier("b".into()),
                TokenKind::Identifier("a".into()),
                TokenKind::FatArrow,
                TokenKind::Identifier("b".into()),
                TokenKind::Identifier("a".into()),
                TokenKind::DotDot,
                TokenKind::Identifier("b".into()),
            ]
        );
    }
}
