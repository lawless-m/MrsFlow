//! Plan IR data model — the logical relational plan and the scalar sub-IR it
//! sits over, per `mrsflow/10-plan-ir.md`.
//!
//! Two trees:
//!   * [`Rel`]    — the relational (table-level) operators.
//!   * [`Scalar`] — the typed expression language inside `each` bodies, the
//!                  layer fold-safety reasoning runs on.
//!
//! Logical only: no node here encodes a backend choice or a fold decision —
//! that is the planner's job, downstream (not yet built). Both layers render
//! to S-expressions (see `sexpr.rs`) so every pass can be dumped and diffed.

/// A node in the logical relational plan. Closed and minimal — one variant per
/// operator in the doc's node-set table, plus [`Rel::EvalM`] as the escape
/// hatch for anything that does not map to a relational operator.
#[derive(Debug, Clone, PartialEq)]
pub enum Rel {
    /// A source/document leaf — `Parquet.Document(p)`, `Odbc.DataSource(..)`,
    /// or a bare reference to a let-bound table.
    Scan(Source),
    /// `Table.SelectRows` — keep rows where `predicate` holds.
    Filter { predicate: Scalar, input: Box<Rel> },
    /// `Table.SelectColumns` / `Table.AddColumn` — the output column list.
    /// When `star` is set the named `items` are *added on top of* the input
    /// columns (the `AddColumn` shape); otherwise `items` *replace* them (the
    /// `SelectColumns` shape).
    Project {
        star: bool,
        items: Vec<ProjectItem>,
        input: Box<Rel>,
    },
    /// `Table.Sort`.
    Sort { keys: Vec<SortKey>, input: Box<Rel> },
    /// `Table.FirstN` / `Table.Range`. `n == None` is an open upper bound
    /// (a pure offset, as in `Table.Range(t, off)`).
    Limit {
        n: Option<u64>,
        offset: u64,
        input: Box<Rel>,
    },
    /// `Table.Group`.
    Aggregate {
        keys: Vec<String>,
        aggs: Vec<Aggregation>,
        input: Box<Rel>,
    },
    /// `Table.NestedJoin`. The doc folds a following `Table.ExpandTableColumn`
    /// into this node (optionally with a flattening `Project`); that collapse
    /// is a later refinement — for now the join itself is captured and the
    /// expand lands as an `EvalM` above it.
    Join {
        kind: JoinKind,
        left_keys: Vec<String>,
        right_keys: Vec<String>,
        left: Box<Rel>,
        right: Box<Rel>,
    },
    /// `Table.Distinct` — `on` empty means "the whole row".
    Distinct { on: Vec<String>, input: Box<Rel> },
    /// The escape hatch: any step that does not map to an operator above.
    /// `descr` names the M operation (for dump-and-diff); `inputs` are the
    /// relational subtrees still visible *below* the boundary — usually the
    /// foldable spine the planner can keep folding underneath the opaque step.
    EvalM { descr: String, inputs: Vec<Rel> },
}

/// What a [`Rel::Scan`] reads from.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// A recognised leaf constructor and its lowered arguments, e.g.
    /// `Parquet.Document("/data/sales.parquet")`.
    Document { func: String, args: Vec<Scalar> },
    /// A bare identifier the lowerer could not resolve to a let-binding —
    /// treated as an opaque scan root (a parameter, a `#shared` name, …).
    Ref(String),
}

/// One output column of a [`Rel::Project`].
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectItem {
    pub name: String,
    pub expr: Scalar,
}

/// One key of a [`Rel::Sort`].
#[derive(Debug, Clone, PartialEq)]
pub struct SortKey {
    pub column: String,
    pub descending: bool,
}

/// One named aggregation of a [`Rel::Aggregate`].
#[derive(Debug, Clone, PartialEq)]
pub struct Aggregation {
    pub name: String,
    pub func: AggFunc,
    /// The column the aggregate ranges over. `None` for a row count and for
    /// opaque aggregations.
    pub column: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggFunc {
    Sum,
    Count,
    Average,
    Min,
    Max,
    CountDistinct,
    /// An `each` body that did not reduce to a recognised aggregate-over-column.
    /// The node still exists (so the plan is faithful) but cannot fold.
    Opaque,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    Inner,
    LeftOuter,
    RightOuter,
    FullOuter,
    LeftAnti,
    RightAnti,
}

/// The scalar (leaf-expression) IR — typed, and distinct from the relational
/// layer. This is where "can this fold to SQL, and is it safe to" gets
/// answered, so it is never a raw M-AST blob.
#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    /// Column reference.
    Col(String),
    /// Table-qualified column reference, rendered `"table"."name"`. Produced
    /// when folding a join, where a bare column name could be ambiguous across
    /// the two sides; unqualified [`Col`](Scalar::Col) stays the common case.
    QualifiedCol { table: String, name: String },
    /// Typed literal — the type is carried (not inferred) so decimal/date/null
    /// reasoning downstream need not re-derive it.
    Lit(Lit),
    Cmp {
        op: CmpOp,
        lhs: Box<Scalar>,
        rhs: Box<Scalar>,
    },
    Bool { op: BoolOp, args: Vec<Scalar> },
    Arith {
        op: ArithOp,
        lhs: Box<Scalar>,
        rhs: Box<Scalar>,
    },
    /// A bounded allow-list of M functions with SQL analogues. Membership is
    /// decided in lowering; everything off the list becomes [`Scalar::Opaque`].
    Call { func: String, args: Vec<Scalar> },
    /// An `each` body that does not reduce to any of the above — a fold
    /// boundary in its own right.
    Opaque,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    /// Raw numeric lexeme, preserved verbatim (as the AST does) so precision
    /// survives to the emitter.
    Number(String),
    Text(String),
    Logical(bool),
    Date(chrono::NaiveDate),
    Datetime(chrono::NaiveDateTime),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    And,
    Or,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
}
