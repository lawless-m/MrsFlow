//! AST for parsed M expressions.
//!
//! Slice 1 covers literals, identifier references, parenthesised expressions
//! (transparent — no AST node), unary `+ - not`, the full binary precedence
//! chain, `if/then/else`, and `let/in`. Function literals, records, lists,
//! field access, invocation, types, `try/otherwise`, `meta`, `each`/`_` are
//! deferred to subsequent parser slices.

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Decimal or hex number literal — raw lexeme preserved; numeric parsing
    /// happens at evaluation time so we don't lose precision in the AST.
    NumberLit(String),
    /// Text literal — fully unescaped value.
    TextLit(String),
    LogicalLit(bool),
    NullLit,

    /// Reference to a name in scope. The string is the identifier as written
    /// (incl. dots for `Table.SelectRows`-style names).
    Identifier(String),

    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),

    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Let {
        bindings: Vec<(String, Expr)>,
        body: Box<Expr>,
    },

    /// Record literal: `[a = 1, b = 2]`. Fields preserve source order.
    Record(Vec<(String, Expr)>),

    /// List literal: `{1, 2, 3, 4..10}`. Items can be single expressions or
    /// ranges; `..` is M's only range operator and only legal here per spec.
    List(Vec<ListItem>),

    /// Function literal: `(x as number, optional y as nullable text) as number => body`.
    /// Each parameter carries an optional flag and an optional type annotation;
    /// the function as a whole has an optional return type.
    Function {
        params: Vec<Param>,
        return_type: Option<Box<Expr>>,
        body: Box<Expr>,
    },

    /// `each E` — surface-syntax sugar for `(_) => E`. Kept as a distinct AST
    /// node so a future pretty-printer can preserve the original form.
    Each(Box<Expr>),

    /// Function invocation: `f(a, b, c)`.
    Invoke {
        target: Box<Expr>,
        args: Vec<Expr>,
    },

    /// Field access: `r[name]` (required) or `r[name]?` (optional, returns
    /// null on missing per spec).
    FieldAccess {
        target: Box<Expr>,
        field: String,
        optional: bool,
    },

    /// Item access: `r{i}` (required) or `r{i}?` (optional, null on missing).
    ItemAccess {
        target: Box<Expr>,
        index: Box<Expr>,
        optional: bool,
    },

    /// `try body` (no fallback) or `try body otherwise fallback`.
    /// Per spec, `try` catches errors and either propagates them as a
    /// special record (no otherwise) or evaluates the fallback (with otherwise).
    Try {
        body: Box<Expr>,
        otherwise: Option<Box<Expr>>,
    },

    /// `error message` — raises an error value at evaluation time.
    Error(Box<Expr>),

    /// `section <name>; <member>; <member>; …` — top-level section document.
    /// Per spec, sections appear only at the root and bundle named bindings
    /// for later reference. mrsflow currently parses them but does not yet
    /// evaluate; the variant is here so the corpus's `whole_section.m`
    /// files round-trip through the parser.
    Section {
        name: String,
        members: Vec<SectionMember>,
    },

    // --- Compound type expressions (only inside `type X` per spec) ---
    //
    // The parser produces these when parsing inside type context. They are
    // distinct from the value-level Record/List variants even though the
    // surface syntax `[...]` and `{...}` overlaps — the disambiguation
    // happens at the parser, driven by being inside a `type X` form.

    /// `type {T}` — list-type with item-type T.
    ListType(Box<Expr>),

    /// `type [a = T, b = T, ...]` — record type. Open if `is_open`, in which
    /// case the field list is non-exhaustive (extra fields allowed).
    RecordType {
        fields: Vec<RecordTypeField>,
        is_open: bool,
    },

    /// `type table T` — table type. Per spec T must be a record-type, but the
    /// parser is lenient; the type-checker enforces.
    TableType(Box<Expr>),

    /// `type function (params) as T` — function type. Reuses `Param` for
    /// parameters; return type is mandatory.
    FunctionType {
        params: Vec<Param>,
        return_type: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectionMember {
    /// True when prefixed with `shared` — visibility marker; ignored at
    /// parse time, may matter for evaluator name resolution later.
    pub shared: bool,
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordTypeField {
    pub name: String,
    pub optional: bool,
    /// `= TYPE` annotation. Per spec the field-type-specification is itself
    /// optional, so a field-spec like `[Name, Age]` is permitted.
    pub type_annotation: Option<Box<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListItem {
    Single(Expr),
    Range(Expr, Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub optional: bool,
    /// `as TYPE` annotation for this parameter. Type-slot grammar applies
    /// (primary-or-nullable-primitive).
    pub type_annotation: Option<Box<Expr>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
    /// `type X` — produce the type value for X.
    Type,
    /// `nullable T` — only valid in type-slot position (RHS of `as`/`is`,
    /// function parameter types, function return types).
    Nullable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Multiplicative
    Multiply,
    Divide,
    // Additive
    Add,
    Subtract,
    /// `&` — text/list concatenation per spec (overloaded by operand type).
    Concat,
    // Relational
    LessThan,
    LessEquals,
    GreaterThan,
    GreaterEquals,
    // Equality
    Equal,
    NotEqual,
    // Logical
    And,
    Or,
    // Type relations — RHS is restricted to a type expression by the parser.
    As,
    Is,
    /// Metadata attachment: `expr meta meta-record`.
    Meta,
}

impl Expr {
    /// Canonical S-expression form, used by the parser-level differential
    /// harness to compare against the Prolog DCG. Format must match
    /// `print_ast/1` in `tools/grammar-fuzz/syntactic.pl` exactly.
    pub fn to_sexpr(&self) -> String {
        let mut out = String::new();
        write_sexpr(&mut out, self);
        out
    }
}

fn write_sexpr(out: &mut String, e: &Expr) {
    match e {
        Expr::NumberLit(n) => {
            out.push_str("(num ");
            write_quoted(out, n);
            out.push(')');
        }
        Expr::TextLit(s) => {
            out.push_str("(text ");
            write_quoted(out, s);
            out.push(')');
        }
        Expr::LogicalLit(true) => out.push_str("(bool true)"),
        Expr::LogicalLit(false) => out.push_str("(bool false)"),
        Expr::NullLit => out.push_str("(null)"),
        Expr::Identifier(n) => {
            out.push_str("(ref ");
            write_quoted(out, n);
            out.push(')');
        }
        Expr::Unary(op, inner) => {
            let name = match op {
                UnaryOp::Plus => "pos",
                UnaryOp::Minus => "neg",
                UnaryOp::Not => "not",
                UnaryOp::Type => "type",
                UnaryOp::Nullable => "nullable",
            };
            out.push('(');
            out.push_str(name);
            out.push(' ');
            write_sexpr(out, inner);
            out.push(')');
        }
        Expr::Binary(op, l, r) => {
            let name = binary_name(*op);
            out.push('(');
            out.push_str(name);
            out.push(' ');
            write_sexpr(out, l);
            out.push(' ');
            write_sexpr(out, r);
            out.push(')');
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            out.push_str("(if ");
            write_sexpr(out, cond);
            out.push(' ');
            write_sexpr(out, then_branch);
            out.push(' ');
            write_sexpr(out, else_branch);
            out.push(')');
        }
        Expr::Let { bindings, body } => {
            out.push_str("(let (");
            for (i, (name, val)) in bindings.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push('(');
                write_quoted(out, name);
                out.push(' ');
                write_sexpr(out, val);
                out.push(')');
            }
            out.push_str(") ");
            write_sexpr(out, body);
            out.push(')');
        }
        Expr::Record(fields) => {
            out.push_str("(record (");
            for (i, (name, val)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push('(');
                write_quoted(out, name);
                out.push(' ');
                write_sexpr(out, val);
                out.push(')');
            }
            out.push_str("))");
        }
        Expr::List(items) => {
            out.push_str("(list (");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                match item {
                    ListItem::Single(e) => {
                        out.push_str("(item ");
                        write_sexpr(out, e);
                        out.push(')');
                    }
                    ListItem::Range(s, e) => {
                        out.push_str("(range ");
                        write_sexpr(out, s);
                        out.push(' ');
                        write_sexpr(out, e);
                        out.push(')');
                    }
                }
            }
            out.push_str("))");
        }
        Expr::Function { params, return_type, body } => {
            // Format: (fn (<param-spec>...) <return-spec> <body>)
            // param-spec: ("name" req|opt none|<type-expr>)
            // return-spec: none | <type-expr>
            out.push_str("(fn (");
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push('(');
                write_quoted(out, &p.name);
                out.push(' ');
                out.push_str(if p.optional { "opt" } else { "req" });
                out.push(' ');
                match &p.type_annotation {
                    None => out.push_str("none"),
                    Some(t) => write_sexpr(out, t),
                }
                out.push(')');
            }
            out.push_str(") ");
            match return_type {
                None => out.push_str("none"),
                Some(t) => write_sexpr(out, t),
            }
            out.push(' ');
            write_sexpr(out, body);
            out.push(')');
        }
        Expr::Each(body) => {
            out.push_str("(each ");
            write_sexpr(out, body);
            out.push(')');
        }
        Expr::Invoke { target, args } => {
            out.push_str("(invoke ");
            write_sexpr(out, target);
            out.push_str(" (");
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                write_sexpr(out, a);
            }
            out.push_str("))");
        }
        Expr::FieldAccess { target, field, optional } => {
            out.push('(');
            out.push_str(if *optional { "field?" } else { "field" });
            out.push(' ');
            write_sexpr(out, target);
            out.push(' ');
            write_quoted(out, field);
            out.push(')');
        }
        Expr::ItemAccess { target, index, optional } => {
            out.push('(');
            out.push_str(if *optional { "item?" } else { "item" });
            out.push(' ');
            write_sexpr(out, target);
            out.push(' ');
            write_sexpr(out, index);
            out.push(')');
        }
        Expr::Try { body, otherwise } => {
            out.push_str("(try ");
            write_sexpr(out, body);
            if let Some(o) = otherwise {
                out.push(' ');
                write_sexpr(out, o);
            }
            out.push(')');
        }
        Expr::Error(message) => {
            out.push_str("(error ");
            write_sexpr(out, message);
            out.push(')');
        }
        Expr::Section { name, members } => {
            out.push_str("(section ");
            write_quoted(out, name);
            out.push_str(" (");
            for (i, m) in members.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push_str("(member ");
                out.push_str(if m.shared { "shared" } else { "private" });
                out.push(' ');
                write_quoted(out, &m.name);
                out.push(' ');
                write_sexpr(out, &m.value);
                out.push(')');
            }
            out.push_str("))");
        }
        Expr::ListType(item) => {
            out.push_str("(list-type ");
            write_sexpr(out, item);
            out.push(')');
        }
        Expr::RecordType { fields, is_open } => {
            out.push_str("(record-type (");
            for (i, f) in fields.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push('(');
                write_quoted(out, &f.name);
                out.push(' ');
                out.push_str(if f.optional { "opt" } else { "req" });
                out.push(' ');
                match &f.type_annotation {
                    None => out.push_str("none"),
                    Some(t) => write_sexpr(out, t),
                }
                out.push(')');
            }
            out.push_str(") ");
            out.push_str(if *is_open { "open" } else { "closed" });
            out.push(')');
        }
        Expr::TableType(row) => {
            out.push_str("(table-type ");
            write_sexpr(out, row);
            out.push(')');
        }
        Expr::FunctionType { params, return_type } => {
            // Same param-spec format as Function literal, but no body and a
            // mandatory return type.
            out.push_str("(function-type (");
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push('(');
                write_quoted(out, &p.name);
                out.push(' ');
                out.push_str(if p.optional { "opt" } else { "req" });
                out.push(' ');
                match &p.type_annotation {
                    None => out.push_str("none"),
                    Some(t) => write_sexpr(out, t),
                }
                out.push(')');
            }
            out.push_str(") ");
            write_sexpr(out, return_type);
            out.push(')');
        }
    }
}

fn binary_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Multiply => "mul",
        BinaryOp::Divide => "div",
        BinaryOp::Add => "add",
        BinaryOp::Subtract => "sub",
        BinaryOp::Concat => "cat",
        BinaryOp::LessThan => "lt",
        BinaryOp::LessEquals => "le",
        BinaryOp::GreaterThan => "gt",
        BinaryOp::GreaterEquals => "ge",
        BinaryOp::Equal => "eq",
        BinaryOp::NotEqual => "ne",
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
        BinaryOp::As => "as",
        BinaryOp::Is => "is",
        BinaryOp::Meta => "meta",
    }
}

/// Quote a string for the S-expression format. Must agree byte-for-byte with
/// the corresponding Prolog helper.
fn write_quoted(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
}
