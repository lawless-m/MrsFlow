//! Recursive-descent parser for the M language.
//!
//! Slice 1 scope is documented on `ast::Expr`. Hand-written; matches the
//! approach of Microsoft's TypeScript reference parser.

mod ast;

pub use ast::{BinaryOp, Expr, ListItem, Param, RecordTypeField, UnaryOp};

use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// A token was found where a different one was expected.
    Unexpected {
        pos: usize,
        found: TokenKind,
        expected: &'static str,
    },
    /// Input ended where more was required.
    UnexpectedEof { expected: &'static str },
}

pub fn parse(tokens: &[Token]) -> Result<Expr, ParseError> {
    let mut p = Parser::new(tokens);
    let expr = p.parse_expression()?;
    if let Some(t) = p.peek() {
        return Err(ParseError::Unexpected {
            pos: t.span.start,
            found: t.kind.clone(),
            expected: "end of input",
        });
    }
    Ok(expr)
}

fn hash_keyword_name(k: &TokenKind) -> Option<&'static str> {
    Some(match k {
        TokenKind::HashBinary => "#binary",
        TokenKind::HashDate => "#date",
        TokenKind::HashDatetime => "#datetime",
        TokenKind::HashDatetimezone => "#datetimezone",
        TokenKind::HashDuration => "#duration",
        TokenKind::HashInfinity => "#infinity",
        TokenKind::HashNan => "#nan",
        TokenKind::HashSections => "#sections",
        TokenKind::HashShared => "#shared",
        TokenKind::HashTable => "#table",
        TokenKind::HashTime => "#time",
        _ => return None,
    })
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.peek().map(|t| &t.kind)
    }

    fn advance(&mut self) -> Option<&'a Token> {
        let t = self.tokens.get(self.pos)?;
        self.pos += 1;
        Some(t)
    }

    fn expect(
        &mut self,
        kind: TokenKind,
        expected: &'static str,
    ) -> Result<(), ParseError> {
        match self.peek() {
            None => Err(ParseError::UnexpectedEof { expected }),
            Some(t) if t.kind == kind => {
                self.pos += 1;
                Ok(())
            }
            Some(t) => Err(ParseError::Unexpected {
                pos: t.span.start,
                found: t.kind.clone(),
                expected,
            }),
        }
    }

    // --- Top-level expression dispatch ---

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        match self.peek_kind() {
            Some(TokenKind::If) => self.parse_if(),
            Some(TokenKind::Let) => self.parse_let(),
            Some(TokenKind::Each) => self.parse_each(),
            Some(TokenKind::Try) => self.parse_try(),
            Some(TokenKind::Error) => self.parse_error_expr(),
            _ => self.parse_logical_or(),
        }
    }

    fn parse_each(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // `each`
        let body = Box::new(self.parse_expression()?);
        Ok(Expr::Each(body))
    }

    fn parse_try(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // `try`
        let body = Box::new(self.parse_expression()?);
        let otherwise = if matches!(self.peek_kind(), Some(TokenKind::Otherwise)) {
            self.advance();
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        Ok(Expr::Try { body, otherwise })
    }

    fn parse_error_expr(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // `error`
        let message = Box::new(self.parse_expression()?);
        Ok(Expr::Error(message))
    }

    fn parse_if(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // `if`
        let cond = Box::new(self.parse_expression()?);
        self.expect(TokenKind::Then, "`then`")?;
        let then_branch = Box::new(self.parse_expression()?);
        self.expect(TokenKind::Else, "`else`")?;
        let else_branch = Box::new(self.parse_expression()?);
        Ok(Expr::If {
            cond,
            then_branch,
            else_branch,
        })
    }

    fn parse_let(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // `let`
        let mut bindings = Vec::new();
        loop {
            let name = self.expect_identifier_name()?;
            self.expect(TokenKind::Equals, "`=`")?;
            let value = self.parse_expression()?;
            bindings.push((name, value));
            if matches!(self.peek_kind(), Some(TokenKind::Comma)) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(TokenKind::In, "`in`")?;
        let body = Box::new(self.parse_expression()?);
        Ok(Expr::Let { bindings, body })
    }

    fn expect_identifier_name(&mut self) -> Result<String, ParseError> {
        match self.peek() {
            None => Err(ParseError::UnexpectedEof {
                expected: "identifier",
            }),
            Some(t) => match &t.kind {
                TokenKind::Identifier(name) => {
                    let n = name.clone();
                    self.pos += 1;
                    Ok(n)
                }
                // Per spec, `identifier` includes quoted-identifier — so let
                // bindings and function parameter names accept `#"with space"`.
                TokenKind::QuotedIdentifier(name) => {
                    let n = name.clone();
                    self.pos += 1;
                    Ok(n)
                }
                other => Err(ParseError::Unexpected {
                    pos: t.span.start,
                    found: other.clone(),
                    expected: "identifier",
                }),
            },
        }
    }

    // --- Binary precedence chain (low → high precedence) ---

    fn parse_logical_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_logical_and()?;
        while matches!(self.peek_kind(), Some(TokenKind::Or)) {
            self.advance();
            let right = self.parse_logical_and()?;
            left = Expr::Binary(BinaryOp::Or, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_is()?;
        while matches!(self.peek_kind(), Some(TokenKind::And)) {
            self.advance();
            let right = self.parse_is()?;
            left = Expr::Binary(BinaryOp::And, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_is(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_as()?;
        while matches!(self.peek_kind(), Some(TokenKind::Is)) {
            self.advance();
            let right = self.parse_primary_type()?;
            left = Expr::Binary(BinaryOp::Is, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_as(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_equality()?;
        while matches!(self.peek_kind(), Some(TokenKind::As)) {
            self.advance();
            let right = self.parse_primary_type()?;
            left = Expr::Binary(BinaryOp::As, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// Parse a type expression. Per spec the strictly-typed positions
    /// (`as`/`is` RHS, function-literal param/return types) accept only
    /// `primitive-or-nullable-primitive-type`; the more permissive positions
    /// (`type X`, compound type sub-positions) accept any `primary-type`.
    /// We use one lenient parser everywhere — semantic restrictions become
    /// the type-checker's job rather than the parser's.
    fn parse_primary_type(&mut self) -> Result<Expr, ParseError> {
        // Contextual prefix keywords: nullable, table, function.
        if self.peek_is_contextual_identifier("nullable") {
            self.advance();
            let inner = self.parse_primary_type()?;
            return Ok(Expr::Unary(UnaryOp::Nullable, Box::new(inner)));
        }
        if self.peek_is_contextual_identifier("table") {
            self.advance();
            let row_type = self.parse_primary_type()?;
            return Ok(Expr::TableType(Box::new(row_type)));
        }
        if self.peek_is_contextual_identifier("function") {
            return self.parse_function_type();
        }

        // Syntactic shapes.
        match self.peek_kind() {
            Some(TokenKind::LeftBrace) => self.parse_list_type(),
            Some(TokenKind::LeftBracket) => self.parse_record_type(),
            Some(TokenKind::LeftParen) => {
                // Parens escape back to expression context per spec —
                // useful for using a variable whose name collides with a
                // primitive-type name, or for invoking a function in type
                // position.
                self.advance();
                let inner = self.parse_expression()?;
                self.expect(TokenKind::RightParen, "`)`")?;
                Ok(inner)
            }
            // Otherwise expect a primitive-type identifier (or any primary).
            _ => self.parse_primary(),
        }
    }

    fn parse_list_type(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // `{`
        let item = self.parse_primary_type()?;
        self.expect(TokenKind::RightBrace, "`}`")?;
        Ok(Expr::ListType(Box::new(item)))
    }

    fn parse_record_type(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // `[`
        let mut fields = Vec::new();
        let mut is_open = false;
        if !matches!(self.peek_kind(), Some(TokenKind::RightBracket)) {
            loop {
                if matches!(self.peek_kind(), Some(TokenKind::Ellipsis)) {
                    self.advance();
                    is_open = true;
                    break;
                }
                let optional = self.peek_is_contextual_identifier("optional");
                if optional {
                    self.advance();
                }
                let name = self.expect_field_name()?;
                let type_annotation = if matches!(self.peek_kind(), Some(TokenKind::Equals)) {
                    self.advance();
                    Some(Box::new(self.parse_primary_type()?))
                } else {
                    None
                };
                fields.push(RecordTypeField {
                    name,
                    optional,
                    type_annotation,
                });
                if matches!(self.peek_kind(), Some(TokenKind::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RightBracket, "`]`")?;
        Ok(Expr::RecordType { fields, is_open })
    }

    fn parse_function_type(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // `function` (contextual identifier)
        self.expect(TokenKind::LeftParen, "`(`")?;
        let mut params = Vec::new();
        if !matches!(self.peek_kind(), Some(TokenKind::RightParen)) {
            params.push(self.parse_param()?);
            while matches!(self.peek_kind(), Some(TokenKind::Comma)) {
                self.advance();
                params.push(self.parse_param()?);
            }
        }
        self.expect(TokenKind::RightParen, "`)`")?;
        self.expect(TokenKind::As, "`as` (function-type return)")?;
        let return_type = self.parse_primary_type()?;
        Ok(Expr::FunctionType {
            params,
            return_type: Box::new(return_type),
        })
    }

    fn peek_is_contextual_identifier(&self, want: &str) -> bool {
        matches!(
            self.peek_kind(),
            Some(TokenKind::Identifier(n)) if n == want
        )
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_relational()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Equals) => BinaryOp::Equal,
                Some(TokenKind::NotEquals) => BinaryOp::NotEqual,
                _ => break,
            };
            self.advance();
            let right = self.parse_relational()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::LessThan) => BinaryOp::LessThan,
                Some(TokenKind::LessEquals) => BinaryOp::LessEquals,
                Some(TokenKind::GreaterThan) => BinaryOp::GreaterThan,
                Some(TokenKind::GreaterEquals) => BinaryOp::GreaterEquals,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Plus) => BinaryOp::Add,
                Some(TokenKind::Minus) => BinaryOp::Subtract,
                Some(TokenKind::Ampersand) => BinaryOp::Concat,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_metadata()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Star) => BinaryOp::Multiply,
                Some(TokenKind::Slash) => BinaryOp::Divide,
                _ => break,
            };
            self.advance();
            let right = self.parse_metadata()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// metadata-expression sits between multiplicative and unary; left-
    /// associative `meta` per spec.
    fn parse_metadata(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        while matches!(self.peek_kind(), Some(TokenKind::Meta)) {
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary(BinaryOp::Meta, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek_kind() {
            Some(TokenKind::Plus) => self.unary_prefix(UnaryOp::Plus),
            Some(TokenKind::Minus) => self.unary_prefix(UnaryOp::Minus),
            Some(TokenKind::Not) => self.unary_prefix(UnaryOp::Not),
            // `type X` switches into type context — X is a primary-type, not
            // a regular unary (per spec: type-expression: ... | type primary-type).
            Some(TokenKind::Type) => {
                self.advance();
                let inner = self.parse_primary_type()?;
                Ok(Expr::Unary(UnaryOp::Type, Box::new(inner)))
            }
            _ => self.parse_postfix(),
        }
    }

    fn unary_prefix(&mut self, op: UnaryOp) -> Result<Expr, ParseError> {
        self.advance();
        let inner = self.parse_unary()?;
        Ok(Expr::Unary(op, Box::new(inner)))
    }

    /// Postfix chain: a primary followed by zero or more invocations,
    /// field-accesses, or item-accesses, all binding tighter than unary.
    /// Each access can be optional (`?` suffix) — returns null on miss per spec.
    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek_kind() {
                Some(TokenKind::LeftParen) => {
                    self.advance();
                    let args = self.parse_argument_list()?;
                    self.expect(TokenKind::RightParen, "`)`")?;
                    expr = Expr::Invoke {
                        target: Box::new(expr),
                        args,
                    };
                }
                Some(TokenKind::LeftBracket) => {
                    self.advance();
                    let field = self.expect_field_name()?;
                    self.expect(TokenKind::RightBracket, "`]`")?;
                    let optional = self.consume_question_mark();
                    expr = Expr::FieldAccess {
                        target: Box::new(expr),
                        field,
                        optional,
                    };
                }
                Some(TokenKind::LeftBrace) => {
                    self.advance();
                    let index = Box::new(self.parse_expression()?);
                    self.expect(TokenKind::RightBrace, "`}`")?;
                    let optional = self.consume_question_mark();
                    expr = Expr::ItemAccess {
                        target: Box::new(expr),
                        index,
                        optional,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_argument_list(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        if matches!(self.peek_kind(), Some(TokenKind::RightParen)) {
            return Ok(args);
        }
        args.push(self.parse_expression()?);
        while matches!(self.peek_kind(), Some(TokenKind::Comma)) {
            self.advance();
            args.push(self.parse_expression()?);
        }
        Ok(args)
    }

    fn consume_question_mark(&mut self) -> bool {
        if matches!(self.peek_kind(), Some(TokenKind::Question)) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Field names accept generalized-identifier (one or more adjacent ident
    /// tokens, joined with single spaces) or a single quoted-identifier.
    /// Used by record literal field names AND field-access selectors.
    fn expect_field_name(&mut self) -> Result<String, ParseError> {
        match self.peek() {
            None => Err(ParseError::UnexpectedEof {
                expected: "field name",
            }),
            Some(t) => match &t.kind {
                TokenKind::QuotedIdentifier(n) => {
                    let n = n.clone();
                    self.pos += 1;
                    Ok(n)
                }
                TokenKind::Identifier(first) => {
                    let mut name = first.clone();
                    self.pos += 1;
                    while let Some(TokenKind::Identifier(more)) = self.peek_kind() {
                        name.push(' ');
                        name.push_str(more);
                        self.pos += 1;
                    }
                    Ok(name)
                }
                other => Err(ParseError::Unexpected {
                    pos: t.span.start,
                    found: other.clone(),
                    expected: "field name",
                }),
            },
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let t = match self.peek() {
            Some(t) => t,
            None => {
                return Err(ParseError::UnexpectedEof {
                    expected: "expression",
                });
            }
        };
        let pos = t.span.start;
        let kind = t.kind.clone();
        match kind {
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expr::NumberLit(n))
            }
            TokenKind::Text(s) => {
                self.advance();
                Ok(Expr::TextLit(s))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::LogicalLit(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::LogicalLit(false))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr::NullLit)
            }
            TokenKind::Identifier(n) => {
                self.advance();
                Ok(Expr::Identifier(n))
            }
            // Quoted identifier `#"..."` is also an identifier reference at
            // primary position — same AST shape as a regular identifier.
            TokenKind::QuotedIdentifier(n) => {
                self.advance();
                Ok(Expr::Identifier(n))
            }
            // `#`-keywords function as identifiers in primary position. The
            // constructor forms (`#date(2024, 1, 1)`, `#table(...)`) become
            // an Invoke wrapping this Identifier; the literal-like forms
            // (`#nan`, `#infinity`, `#sections`, `#shared`) stand alone and
            // the evaluator resolves them in the intrinsic environment.
            ref k if hash_keyword_name(k).is_some() => {
                let name = hash_keyword_name(k).unwrap();
                self.advance();
                Ok(Expr::Identifier(name.into()))
            }
            TokenKind::LeftParen => self.parse_parens_or_function(),
            TokenKind::LeftBracket => self.parse_bracketed_primary(),
            TokenKind::LeftBrace => self.parse_list(),
            other => Err(ParseError::Unexpected {
                pos,
                found: other,
                expected: "expression",
            }),
        }
    }

    /// Parenthesised expression or function literal — disambiguated by looking
    /// ahead for the matching `)` and checking if `=>` follows.
    fn parse_parens_or_function(&mut self) -> Result<Expr, ParseError> {
        if self.looks_like_function_literal() {
            return self.parse_function();
        }
        self.advance(); // `(`
        let inner = self.parse_expression()?;
        self.expect(TokenKind::RightParen, "`)`")?;
        Ok(inner)
    }

    /// Cursor is on `(`. Walk balanced parens to the matching close, then
    /// check whether what follows could be a function-literal tail:
    /// either `=>` directly or `as TYPE =>` for a return-type annotation.
    fn looks_like_function_literal(&self) -> bool {
        let mut depth: i32 = 0;
        for (i, t) in self.tokens[self.pos..].iter().enumerate() {
            match t.kind {
                TokenKind::LeftParen => depth += 1,
                TokenKind::RightParen => {
                    depth -= 1;
                    if depth == 0 {
                        return self.has_function_arrow_after(self.pos + i + 1);
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn has_function_arrow_after(&self, idx: usize) -> bool {
        match self.tokens.get(idx).map(|t| &t.kind) {
            Some(TokenKind::FatArrow) => true,
            // `as TYPE =>` — TYPE may be primitive, nullable, or a compound
            // type expression. We scan forward at depth 0 looking for `=>`,
            // bracket-aware so `as table [a = number] =>` works.
            Some(TokenKind::As) => self.scan_for_fat_arrow(idx + 1, 64),
            _ => false,
        }
    }

    /// Scan forward up to `max` tokens for a top-level `=>`, tracking nesting
    /// of `()`, `[]`, `{}`. Returns true if `=>` is found at depth 0.
    fn scan_for_fat_arrow(&self, start: usize, max: usize) -> bool {
        let mut depth: i32 = 0;
        for i in 0..max {
            match self.tokens.get(start + i).map(|t| &t.kind) {
                None => return false,
                Some(TokenKind::FatArrow) if depth == 0 => return true,
                Some(TokenKind::LeftParen)
                | Some(TokenKind::LeftBracket)
                | Some(TokenKind::LeftBrace) => depth += 1,
                Some(TokenKind::RightParen)
                | Some(TokenKind::RightBracket)
                | Some(TokenKind::RightBrace) => {
                    depth -= 1;
                    if depth < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn parse_function(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // `(`
        let mut params = Vec::new();
        if !matches!(self.peek_kind(), Some(TokenKind::RightParen)) {
            params.push(self.parse_param()?);
            while matches!(self.peek_kind(), Some(TokenKind::Comma)) {
                self.advance();
                params.push(self.parse_param()?);
            }
        }
        self.expect(TokenKind::RightParen, "`)`")?;
        let return_type = if matches!(self.peek_kind(), Some(TokenKind::As)) {
            self.advance();
            Some(Box::new(self.parse_primary_type()?))
        } else {
            None
        };
        self.expect(TokenKind::FatArrow, "`=>`")?;
        let body = Box::new(self.parse_expression()?);
        Ok(Expr::Function {
            params,
            return_type,
            body,
        })
    }

    fn parse_param(&mut self) -> Result<Param, ParseError> {
        let optional = if self.peek_is_contextual_identifier("optional") {
            self.advance();
            true
        } else {
            false
        };
        let name = self.expect_identifier_name()?;
        let type_annotation = if matches!(self.peek_kind(), Some(TokenKind::As)) {
            self.advance();
            Some(Box::new(self.parse_primary_type()?))
        } else {
            None
        };
        Ok(Param {
            name,
            optional,
            type_annotation,
        })
    }

    /// Disambiguate the four forms that begin with `[`:
    ///   `[]`           → empty record
    ///   `[name]` `[name]?`  → implicit field access on `_`
    ///   `[name1 name2 …]` → implicit field access on `_` w/ generalized id
    ///   `[name = …]`   → record literal (incl. quoted/generalized name)
    fn parse_bracketed_primary(&mut self) -> Result<Expr, ParseError> {
        // Walk past the field-name shape (1+ identifier tokens OR a single
        // quoted identifier) and check what follows: `]` → implicit access,
        // anything else → record literal (which will expect `=` after the
        // first name).
        let mut i = self.pos + 1;
        match self.tokens.get(i).map(|t| &t.kind) {
            Some(TokenKind::QuotedIdentifier(_)) => i += 1,
            Some(TokenKind::Identifier(_)) => {
                i += 1;
                while matches!(
                    self.tokens.get(i).map(|t| &t.kind),
                    Some(TokenKind::Identifier(_))
                ) {
                    i += 1;
                }
            }
            _ => return self.parse_record(),
        }
        if matches!(
            self.tokens.get(i).map(|t| &t.kind),
            Some(TokenKind::RightBracket)
        ) {
            self.advance(); // `[`
            let name = self.expect_field_name()?;
            self.expect(TokenKind::RightBracket, "`]`")?;
            let optional = self.consume_question_mark();
            Ok(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("_".into())),
                field: name,
                optional,
            })
        } else {
            self.parse_record()
        }
    }

    fn parse_record(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // `[`
        let mut fields = Vec::new();
        if !matches!(self.peek_kind(), Some(TokenKind::RightBracket)) {
            loop {
                let name = self.expect_field_name()?;
                self.expect(TokenKind::Equals, "`=`")?;
                let value = self.parse_expression()?;
                fields.push((name, value));
                if matches!(self.peek_kind(), Some(TokenKind::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RightBracket, "`]`")?;
        Ok(Expr::Record(fields))
    }

    fn parse_list(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // `{`
        let mut items = Vec::new();
        if !matches!(self.peek_kind(), Some(TokenKind::RightBrace)) {
            loop {
                let first = self.parse_expression()?;
                let item = if matches!(self.peek_kind(), Some(TokenKind::DotDot)) {
                    self.advance();
                    let end = self.parse_expression()?;
                    ListItem::Range(first, end)
                } else {
                    ListItem::Single(first)
                };
                items.push(item);
                if matches!(self.peek_kind(), Some(TokenKind::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RightBrace, "`}`")?;
        Ok(Expr::List(items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn p(src: &str) -> Expr {
        let toks = tokenize(src).expect("lex");
        parse(&toks).expect("parse")
    }

    fn s(src: &str) -> String {
        p(src).to_sexpr()
    }

    #[test]
    fn literal_number() {
        assert_eq!(s("42"), r#"(num "42")"#);
        assert_eq!(s("0xff"), r#"(num "0xff")"#);
        assert_eq!(s("3.14"), r#"(num "3.14")"#);
    }

    #[test]
    fn literal_text() {
        assert_eq!(s(r#""hi""#), r#"(text "hi")"#);
    }

    #[test]
    fn literal_logical_and_null() {
        assert_eq!(s("true"), "(bool true)");
        assert_eq!(s("false"), "(bool false)");
        assert_eq!(s("null"), "(null)");
    }

    #[test]
    fn identifier_ref() {
        assert_eq!(s("foo"), r#"(ref "foo")"#);
        assert_eq!(s("Table.SelectRows"), r#"(ref "Table.SelectRows")"#);
    }

    #[test]
    fn parens_are_transparent() {
        // `(42)` should produce the same AST as `42`.
        assert_eq!(s("(42)"), s("42"));
        assert_eq!(s("((42))"), s("42"));
    }

    #[test]
    fn unary_operators() {
        assert_eq!(s("-x"), r#"(neg (ref "x"))"#);
        assert_eq!(s("+x"), r#"(pos (ref "x"))"#);
        assert_eq!(s("not x"), r#"(not (ref "x"))"#);
        assert_eq!(s("--x"), r#"(neg (neg (ref "x")))"#);
    }

    #[test]
    fn additive_left_associative() {
        // 1 - 2 - 3 must parse as (1 - 2) - 3, not 1 - (2 - 3).
        assert_eq!(
            s("1 - 2 - 3"),
            r#"(sub (sub (num "1") (num "2")) (num "3"))"#
        );
    }

    #[test]
    fn multiplicative_binds_tighter_than_additive() {
        // 1 + 2 * 3 = 1 + (2 * 3)
        assert_eq!(
            s("1 + 2 * 3"),
            r#"(add (num "1") (mul (num "2") (num "3")))"#
        );
    }

    #[test]
    fn unary_binds_tighter_than_multiplicative() {
        // -2 * 3 = (-2) * 3, not -(2 * 3)
        assert_eq!(
            s("-2 * 3"),
            r#"(mul (neg (num "2")) (num "3"))"#
        );
    }

    #[test]
    fn equality_and_relational_precedence() {
        // a < b = c parses as (a < b) = c per spec (relational > equality).
        assert_eq!(
            s("a < b = c"),
            r#"(eq (lt (ref "a") (ref "b")) (ref "c"))"#
        );
    }

    #[test]
    fn logical_or_lower_than_and() {
        // a or b and c = a or (b and c)
        assert_eq!(
            s("a or b and c"),
            r#"(or (ref "a") (and (ref "b") (ref "c")))"#
        );
    }

    #[test]
    fn concat_operator() {
        assert_eq!(
            s(r#""hello" & " " & "world""#),
            r#"(cat (cat (text "hello") (text " ")) (text "world"))"#
        );
    }

    #[test]
    fn if_then_else() {
        assert_eq!(
            s("if x > 0 then 1 else -1"),
            r#"(if (gt (ref "x") (num "0")) (num "1") (neg (num "1")))"#
        );
    }

    #[test]
    fn nested_if() {
        assert_eq!(
            s("if a then if b then 1 else 2 else 3"),
            r#"(if (ref "a") (if (ref "b") (num "1") (num "2")) (num "3"))"#
        );
    }

    #[test]
    fn let_single_binding() {
        assert_eq!(
            s("let x = 1 in x"),
            r#"(let (("x" (num "1"))) (ref "x"))"#
        );
    }

    #[test]
    fn let_multiple_bindings() {
        assert_eq!(
            s("let a = 1, b = 2 in a + b"),
            r#"(let (("a" (num "1")) ("b" (num "2"))) (add (ref "a") (ref "b")))"#
        );
    }

    #[test]
    fn let_with_if_body() {
        assert_eq!(
            s("let x = 1 in if x > 0 then x else -x"),
            r#"(let (("x" (num "1"))) (if (gt (ref "x") (num "0")) (ref "x") (neg (ref "x"))))"#
        );
    }

    #[test]
    fn missing_then_errors() {
        let toks = tokenize("if x 1 else 2").unwrap();
        assert!(matches!(
            parse(&toks),
            Err(ParseError::Unexpected { expected: "`then`", .. })
        ));
    }

    #[test]
    fn missing_in_errors() {
        let toks = tokenize("let x = 1").unwrap();
        assert!(matches!(
            parse(&toks),
            Err(ParseError::UnexpectedEof { expected: "`in`" })
        ));
    }

    #[test]
    fn trailing_garbage_errors() {
        let toks = tokenize("1 + 2 garbage").unwrap();
        assert!(matches!(
            parse(&toks),
            Err(ParseError::Unexpected { expected: "end of input", .. })
        ));
    }

    #[test]
    fn sexpr_quotes_special_chars() {
        let e = Expr::TextLit("he said \"hi\"\n".to_string());
        assert_eq!(e.to_sexpr(), r#"(text "he said \"hi\"\n")"#);
    }

    #[test]
    fn record_literal() {
        assert_eq!(
            s("[a = 1, b = 2]"),
            r#"(record (("a" (num "1")) ("b" (num "2"))))"#
        );
    }

    #[test]
    fn empty_record() {
        assert_eq!(s("[]"), "(record ())");
    }

    #[test]
    fn record_with_quoted_field_name() {
        assert_eq!(
            s(r##"[#"with space" = 1]"##),
            r#"(record (("with space" (num "1"))))"#
        );
    }

    #[test]
    fn list_literal() {
        assert_eq!(
            s("{1, 2, 3}"),
            r#"(list ((item (num "1")) (item (num "2")) (item (num "3"))))"#
        );
    }

    #[test]
    fn empty_list() {
        assert_eq!(s("{}"), "(list ())");
    }

    #[test]
    fn list_with_range() {
        assert_eq!(
            s("{1..10}"),
            r#"(list ((range (num "1") (num "10"))))"#
        );
        assert_eq!(
            s("{1, 2..5, 9}"),
            r#"(list ((item (num "1")) (range (num "2") (num "5")) (item (num "9"))))"#
        );
    }

    #[test]
    fn function_literal_no_args() {
        assert_eq!(s("() => 42"), r#"(fn () none (num "42"))"#);
    }

    #[test]
    fn function_literal_one_arg() {
        assert_eq!(
            s("(x) => x + 1"),
            r#"(fn (("x" req none)) none (add (ref "x") (num "1")))"#
        );
    }

    #[test]
    fn function_literal_multiple_args() {
        assert_eq!(
            s("(x, y) => x + y"),
            r#"(fn (("x" req none) ("y" req none)) none (add (ref "x") (ref "y")))"#
        );
    }

    #[test]
    fn nested_function_literals() {
        assert_eq!(
            s("(x) => (y) => x + y"),
            r#"(fn (("x" req none)) none (fn (("y" req none)) none (add (ref "x") (ref "y"))))"#
        );
    }

    #[test]
    fn function_literal_typed_param() {
        assert_eq!(
            s("(x as number) => x + 1"),
            r#"(fn (("x" req (ref "number"))) none (add (ref "x") (num "1")))"#
        );
    }

    #[test]
    fn function_literal_optional_param() {
        assert_eq!(
            s("(x, optional y) => x"),
            r#"(fn (("x" req none) ("y" opt none)) none (ref "x"))"#
        );
    }

    #[test]
    fn function_literal_optional_typed() {
        assert_eq!(
            s("(x as number, optional y as text) => x"),
            r#"(fn (("x" req (ref "number")) ("y" opt (ref "text"))) none (ref "x"))"#
        );
    }

    #[test]
    fn function_literal_with_return_type() {
        assert_eq!(
            s("(x) as number => x"),
            r#"(fn (("x" req none)) (ref "number") (ref "x"))"#
        );
    }

    #[test]
    fn function_literal_nullable_param_and_return() {
        assert_eq!(
            s("(x as nullable number) as nullable text => x"),
            r#"(fn (("x" req (nullable (ref "number")))) (nullable (ref "text")) (ref "x"))"#
        );
    }

    #[test]
    fn nullable_in_as_operator() {
        assert_eq!(
            s("x as nullable number"),
            r#"(as (ref "x") (nullable (ref "number")))"#
        );
    }

    #[test]
    fn nullable_in_is_operator() {
        assert_eq!(
            s("x is nullable text"),
            r#"(is (ref "x") (nullable (ref "text")))"#
        );
    }

    #[test]
    fn parens_still_work_after_function_introduction() {
        // `(x)` alone is parens, not a function — no `=>` follows.
        assert_eq!(s("(x)"), r#"(ref "x")"#);
        assert_eq!(s("(1 + 2)"), s("1 + 2"));
    }

    #[test]
    fn each_expression() {
        assert_eq!(
            s("each x + 1"),
            r#"(each (add (ref "x") (num "1")))"#
        );
        assert_eq!(s("each _"), r#"(each (ref "_"))"#);
    }

    #[test]
    fn invocation() {
        assert_eq!(
            s("f(1, 2, 3)"),
            r#"(invoke (ref "f") ((num "1") (num "2") (num "3")))"#
        );
    }

    #[test]
    fn invocation_zero_args() {
        assert_eq!(
            s("now()"),
            r#"(invoke (ref "now") ())"#
        );
    }

    #[test]
    fn dotted_function_invocation() {
        assert_eq!(
            s(r#"Table.SelectRows(t, each true)"#),
            r#"(invoke (ref "Table.SelectRows") ((ref "t") (each (bool true))))"#
        );
    }

    #[test]
    fn field_access() {
        assert_eq!(
            s("r[Name]"),
            r#"(field (ref "r") "Name")"#
        );
    }

    #[test]
    fn field_access_optional() {
        assert_eq!(
            s("r[Name]?"),
            r#"(field? (ref "r") "Name")"#
        );
    }

    #[test]
    fn field_access_with_quoted_name() {
        assert_eq!(
            s(r##"r[#"with space"]"##),
            r#"(field (ref "r") "with space")"#
        );
    }

    #[test]
    fn item_access() {
        assert_eq!(
            s("xs{0}"),
            r#"(item (ref "xs") (num "0"))"#
        );
        assert_eq!(
            s("xs{i + 1}?"),
            r#"(item? (ref "xs") (add (ref "i") (num "1")))"#
        );
    }

    #[test]
    fn postfix_chain() {
        // f(x)[name]{0}
        assert_eq!(
            s("f(x)[name]{0}"),
            r#"(item (field (invoke (ref "f") ((ref "x"))) "name") (num "0"))"#
        );
    }

    #[test]
    fn invoke_a_function_literal() {
        assert_eq!(
            s("((x) => x * 2)(5)"),
            r#"(invoke (fn (("x" req none)) none (mul (ref "x") (num "2"))) ((num "5")))"#
        );
    }

    #[test]
    fn implicit_field_access() {
        // `[name]` at primary position desugars to `_[name]`, used inside
        // `each` bodies and similar contexts. Per spec implicit-target-
        // field-selector is a primary-expression alternative.
        assert_eq!(
            s("[a]"),
            r#"(field (ref "_") "a")"#
        );
        assert_eq!(
            s("[a]?"),
            r#"(field? (ref "_") "a")"#
        );
        assert_eq!(
            s("each [a] > 0"),
            r#"(each (gt (field (ref "_") "a") (num "0")))"#
        );
    }

    #[test]
    fn hash_keyword_as_intrinsic_ref() {
        // The literal-like #-keywords stand alone as references to intrinsics.
        assert_eq!(s("#nan"), r##"(ref "#nan")"##);
        assert_eq!(s("#infinity"), r##"(ref "#infinity")"##);
        assert_eq!(s("#sections"), r##"(ref "#sections")"##);
        assert_eq!(s("#shared"), r##"(ref "#shared")"##);
    }

    #[test]
    fn hash_keyword_constructor_invocation() {
        assert_eq!(
            s("#date(2024, 1, 1)"),
            r##"(invoke (ref "#date") ((num "2024") (num "1") (num "1")))"##
        );
        assert_eq!(
            s(r##"#table({"a"}, {{1}})"##),
            r##"(invoke (ref "#table") ((list ((item (text "a")))) (list ((item (list ((item (num "1")))))))))"##
        );
    }

    #[test]
    fn try_without_otherwise() {
        assert_eq!(
            s("try foo()"),
            r#"(try (invoke (ref "foo") ()))"#
        );
    }

    #[test]
    fn try_with_otherwise() {
        assert_eq!(
            s("try foo() otherwise null"),
            r#"(try (invoke (ref "foo") ()) (null))"#
        );
    }

    #[test]
    fn error_expression() {
        assert_eq!(
            s(r#"error "bad input""#),
            r#"(error (text "bad input"))"#
        );
    }

    #[test]
    fn try_otherwise_in_let() {
        assert_eq!(
            s("let x = try parse() otherwise 0 in x"),
            r#"(let (("x" (try (invoke (ref "parse") ()) (num "0")))) (ref "x"))"#
        );
    }

    #[test]
    fn as_operator() {
        assert_eq!(
            s("x as number"),
            r#"(as (ref "x") (ref "number"))"#
        );
    }

    #[test]
    fn is_operator() {
        assert_eq!(
            s("x is text"),
            r#"(is (ref "x") (ref "text"))"#
        );
    }

    #[test]
    fn as_left_associative() {
        // x as a as b → (x as a) as b
        assert_eq!(
            s("x as a as b"),
            r#"(as (as (ref "x") (ref "a")) (ref "b"))"#
        );
    }

    #[test]
    fn is_binds_looser_than_as() {
        // x as a is b → (x as a) is b
        assert_eq!(
            s("x as a is b"),
            r#"(is (as (ref "x") (ref "a")) (ref "b"))"#
        );
    }

    #[test]
    fn as_binds_tighter_than_equality() {
        // Per spec, `as` is between equality and is. So `x as T = 5` doesn't
        // parse as a comparison — the `=` is leftover and trailing-garbage
        // errors. User must paren as `(x as T) = 5`.
        let toks = tokenize("x as number = 5").unwrap();
        assert!(matches!(parse(&toks), Err(_)));
        assert_eq!(
            s("(x as number) = 5"),
            r#"(eq (as (ref "x") (ref "number")) (num "5"))"#
        );
    }

    #[test]
    fn type_unary_prefix() {
        assert_eq!(
            s("type number"),
            r#"(type (ref "number"))"#
        );
    }

    #[test]
    fn type_inside_invocation() {
        assert_eq!(
            s(r#"Table.AddColumn(t, "x", each [a] * 2, type number)"#),
            r#"(invoke (ref "Table.AddColumn") ((ref "t") (text "x") (each (mul (field (ref "_") "a") (num "2"))) (type (ref "number"))))"#
        );
    }

    #[test]
    fn meta_operator() {
        assert_eq!(
            s("x meta y"),
            r#"(meta (ref "x") (ref "y"))"#
        );
    }

    #[test]
    fn meta_left_associative() {
        // a meta b meta c → (a meta b) meta c
        assert_eq!(
            s("a meta b meta c"),
            r#"(meta (meta (ref "a") (ref "b")) (ref "c"))"#
        );
    }

    #[test]
    fn meta_binds_tighter_than_multiplicative() {
        // a meta b * c → (a meta b) * c
        assert_eq!(
            s("a meta b * c"),
            r#"(mul (meta (ref "a") (ref "b")) (ref "c"))"#
        );
    }

    #[test]
    fn generalized_identifier_in_record_field() {
        assert_eq!(
            s("[Base Line = 100]"),
            r#"(record (("Base Line" (num "100"))))"#
        );
    }

    #[test]
    fn generalized_identifier_in_field_access() {
        assert_eq!(
            s("r[Base Line]"),
            r#"(field (ref "r") "Base Line")"#
        );
    }

    #[test]
    fn generalized_identifier_implicit_access() {
        // `[Base Line]` standalone is implicit field access on `_` for
        // generalized name "Base Line".
        assert_eq!(
            s("[Base Line]"),
            r#"(field (ref "_") "Base Line")"#
        );
        assert_eq!(
            s("each [Base Line] > 0"),
            r#"(each (gt (field (ref "_") "Base Line") (num "0")))"#
        );
    }

    #[test]
    fn quoted_identifier_in_let_binding() {
        assert_eq!(
            s(r##"let #"with space" = 1 in #"with space""##),
            r#"(let (("with space" (num "1"))) (ref "with space"))"#
        );
    }

    #[test]
    fn type_list() {
        assert_eq!(
            s("type {number}"),
            r#"(type (list-type (ref "number")))"#
        );
        assert_eq!(
            s("type {{text}}"),
            r#"(type (list-type (list-type (ref "text"))))"#
        );
    }

    #[test]
    fn type_record_closed() {
        assert_eq!(
            s("type [a = number, b = text]"),
            r#"(type (record-type (("a" req (ref "number")) ("b" req (ref "text"))) closed))"#
        );
    }

    #[test]
    fn type_record_open() {
        assert_eq!(
            s("type [a = number, ...]"),
            r#"(type (record-type (("a" req (ref "number"))) open))"#
        );
        // Pure open marker with no fields.
        assert_eq!(
            s("type [...]"),
            r#"(type (record-type () open))"#
        );
    }

    #[test]
    fn type_record_optional_field() {
        assert_eq!(
            s("type [a = number, optional b = text]"),
            r#"(type (record-type (("a" req (ref "number")) ("b" opt (ref "text"))) closed))"#
        );
    }

    #[test]
    fn type_record_field_without_type() {
        // Spec allows field-spec without `= type`.
        assert_eq!(
            s("type [a, b]"),
            r#"(type (record-type (("a" req none) ("b" req none)) closed))"#
        );
    }

    #[test]
    fn type_table() {
        assert_eq!(
            s("type table [a = number, b = text]"),
            r#"(type (table-type (record-type (("a" req (ref "number")) ("b" req (ref "text"))) closed)))"#
        );
    }

    #[test]
    fn type_function() {
        assert_eq!(
            s("type function (x as number) as number"),
            r#"(type (function-type (("x" req (ref "number"))) (ref "number")))"#
        );
    }

    #[test]
    fn type_function_with_optional_and_compound_return() {
        assert_eq!(
            s("type function (x as number, optional y as text) as nullable text"),
            r#"(type (function-type (("x" req (ref "number")) ("y" opt (ref "text"))) (nullable (ref "text"))))"#
        );
    }

    #[test]
    fn type_nested_compound() {
        // List of records of typed fields.
        assert_eq!(
            s("type {[Name = text, Age = number]}"),
            r#"(type (list-type (record-type (("Name" req (ref "text")) ("Age" req (ref "number"))) closed)))"#
        );
    }

    #[test]
    fn type_paren_escape() {
        // Per spec, `(...)` inside type context escapes back to expression
        // context. `(myType)` here is just a primary-expression reference to
        // the variable `myType`.
        assert_eq!(
            s("type {(myType)}"),
            r#"(type (list-type (ref "myType")))"#
        );
    }

    #[test]
    fn function_literal_with_compound_return_type() {
        // Tests that the function-literal lookahead handles `as table [...]`
        // before the `=>`.
        assert_eq!(
            s("(t) as table [a = number] => t"),
            r#"(fn (("t" req none)) (table-type (record-type (("a" req (ref "number"))) closed)) (ref "t"))"#
        );
    }

    #[test]
    fn realistic_table_pipeline() {
        // Now that slice 3 makes #table an identifier-like reference, the
        // pipeline parses end-to-end. Asserts the outer shape is a let with
        // two bindings and a body referencing the second.
        let src = r#"let
            t = #table({"a", "b"}, {{1, 2}, {3, 4}}),
            filtered = Table.SelectRows(t, each [a] > 1)
        in filtered"#;
        let ast = p(src);
        if let Expr::Let { bindings, body } = ast {
            assert_eq!(bindings.len(), 2);
            assert_eq!(bindings[0].0, "t");
            assert_eq!(bindings[1].0, "filtered");
            assert_eq!(*body, Expr::Identifier("filtered".into()));
        } else {
            panic!("expected let expression at top level, got {:?}", ast);
        }
    }
}
