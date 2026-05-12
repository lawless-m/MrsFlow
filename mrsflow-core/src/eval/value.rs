//! Value representation for the M evaluator.
//!
//! See `mrsflow/07-evaluator-design.md` §4 for the full variant list. Variants
//! for kinds not yet needed by a landed slice use placeholder payloads (e.g.
//! `String` for the date types until chrono lands, a tiny `Table` struct until
//! Arrow does). They exist in the enum so evaluator code can pattern-match
//! exhaustively from day one and so the type's shape doesn't change
//! disruptively as later slices land.

use std::cell::RefCell;
use std::fmt;
use std::rc::{Rc, Weak};

use crate::parser::{Expr, Param};

use super::env::{Env, EnvNode};
use super::iohost::IoHost;

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Logical(bool),
    Number(f64),
    Text(String),
    Date(chrono::NaiveDate),
    /// Naive (timezone-less) datetime.
    Datetime(chrono::NaiveDateTime),
    /// Tz-bearing datetime — DateTime with a fixed UTC offset.
    Datetimezone(chrono::DateTime<chrono::FixedOffset>),
    Time(chrono::NaiveTime),
    Duration(chrono::Duration),
    Binary(Vec<u8>),
    List(Vec<Value>),
    /// Records preserve insertion order per spec — `Vec` not `HashMap`.
    /// The `env` field keeps the per-record thunk env alive so sibling-field
    /// references resolve correctly when fields are forced after the record
    /// has escaped its construction scope.
    Record(Record),
    /// Placeholder — `arrow::RecordBatch` when the Arrow dep lands in eval-7.
    Table(Table),
    Function(Closure),
    Type(TypeRep),
    /// `value meta record` attaches a metadata record. Per the spec
    /// metadata is preserved through field/item access but not through
    /// arithmetic. Most operations should strip-and-rewrap or strip-only,
    /// using `Value::strip_metadata` to peek at the inner value.
    WithMetadata {
        inner: Box<Value>,
        meta: Record,
    },
    /// Lazy thunk — forced on first access, memoised thereafter. Central to
    /// M's laziness (per design doc §07 §1).
    Thunk(Rc<RefCell<ThunkState>>),
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub params: Vec<Param>,
    pub body: FnBody,
    pub env: Env,
}

/// A function's body is either an M expression (user-defined closures, `each`
/// desugaring, function literals) or a native Rust function pointer (the
/// stdlib intrinsics bound in the root env).
#[derive(Clone)]
pub enum FnBody {
    M(Box<Expr>),
    Builtin(BuiltinFn),
}

impl std::fmt::Debug for FnBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FnBody::M(expr) => f.debug_tuple("M").field(expr).finish(),
            FnBody::Builtin(_) => f.write_str("Builtin(<fn>)"),
        }
    }
}

/// Signature for an intrinsic. The host gives IO-mediated builtins (Parquet,
/// ODBC, Web, …) access to the shell. Pure builtins ignore the second
/// argument.
pub type BuiltinFn = fn(&[Value], &dyn IoHost) -> Result<Value, MError>;

#[derive(Debug, Clone)]
pub struct Record {
    pub fields: Vec<(String, Value)>,
    /// Strong reference to the per-record thunk env. Each thunk in `fields`
    /// holds a `Weak<EnvNode>` back at this env (to break the env↔thunk
    /// reference cycle); this `Rc` keeps the env alive until the record is
    /// dropped, so field thunks remain forceable after the record escapes
    /// its construction scope.
    pub env: Env,
}

/// Type-values produced by `type X`. Primitive types + the `nullable T`
/// wrapper land here, along with the four structural variants:
/// `type {T}` → ListOf, `type [a = T]` → RecordOf, `type table [...]` →
/// TableOf, `type function (...) as T` → FunctionOf.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeRep {
    Any,
    AnyNonNull,
    Null,
    Logical,
    Number,
    Text,
    Date,
    Datetime,
    Datetimezone,
    Time,
    Duration,
    Binary,
    List,
    Record,
    Table,
    Function,
    Type,
    Nullable(Box<TypeRep>),
    /// `type {T}` — list of T.
    ListOf(Box<TypeRep>),
    /// `type [a = T, b = optional T, …]` (closed) or with `…` trailing (open).
    RecordOf {
        fields: Vec<(String, TypeRep, bool /* optional */)>,
        open: bool,
    },
    /// `type table [a = T, b = T]` — table whose row-record matches.
    TableOf {
        columns: Vec<(String, TypeRep)>,
    },
    /// `type function (n as T, optional t as U) as R` — function type.
    FunctionOf {
        params: Vec<(TypeRep, bool /* optional */)>,
        return_type: Box<TypeRep>,
    },
}

/// State of a lazy thunk: pending evaluation (with the captured expression
/// and a *weak* reference to the environment) or already forced to a concrete
/// value (memoised).
///
/// The env reference is `Weak<EnvNode>` rather than `Rc<EnvNode>` to break
/// the reference cycle between an env and the thunks it stores. The env
/// stays alive while the let-body is being evaluated (the body holds an Rc),
/// and any forced values that escape (e.g. closures) hold their own Rcs to
/// keep their captured envs alive.
///
/// `Native` variant — for host-driven deferred work where there's no Expr
/// to evaluate (e.g. Odbc.DataSource navigation tables, where the `Data`
/// cell only fires its SELECT when forced). The closure runs once;
/// memoisation happens via the surrounding RefCell flipping to `Forced`.
pub enum ThunkState {
    Pending { expr: Expr, env: Weak<EnvNode> },
    Native(NativeThunkFn),
    /// Cycle sentinel: this thunk's evaluation is in progress on the
    /// stack. Re-entering force on the same thunk while it's `Forcing`
    /// means the expression depends on itself — raise an error instead
    /// of recursing forever. Restored to `Forced(value)` when the
    /// outer force completes.
    Forcing,
    Forced(Value),
}

/// A no-argument callback returning a forced Value. `Rc<dyn Fn>` (not
/// `FnOnce`) so the type-id is uniform; in practice each closure runs at
/// most once because the surrounding `RefCell<ThunkState>` flips to
/// `Forced` after the first invocation.
pub type NativeThunkFn = Rc<dyn Fn() -> Result<Value, MError>>;

impl fmt::Debug for ThunkState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThunkState::Pending { expr, env } => f
                .debug_struct("Pending")
                .field("expr", expr)
                .field("env", env)
                .finish(),
            ThunkState::Native(_) => f.write_str("Native(<fn>)"),
            ThunkState::Forcing => f.write_str("Forcing"),
            ThunkState::Forced(v) => f.debug_tuple("Forced").field(v).finish(),
        }
    }
}

/// Table representation. Two backings: Arrow `RecordBatch` (typed, uniform
/// columns — what we use for Parquet pipelines and typed-cast operations),
/// or a row-list with per-cell `Value` (heterogeneous: mixed-primitive
/// columns, nested Record/Table/List cells — the M-shaped fallback).
///
/// Slice 1 of the het-cell refactor: the enum exists but only Arrow is
/// constructed; Rows is wired into the type system for later slices to
/// populate. Existing behaviour is preserved.
#[derive(Debug, Clone)]
pub enum TableRepr {
    Arrow(arrow::record_batch::RecordBatch),
    Rows {
        columns: Vec<String>,
        rows: Vec<Vec<Value>>,
    },
}

/// Table value — wraps a [`TableRepr`]. Use the inherent helpers
/// (`column_names`, `num_rows`, `as_arrow`, `try_to_arrow`) instead of
/// reaching into the variant directly.
#[derive(Debug, Clone)]
pub struct Table {
    pub repr: TableRepr,
}

impl Table {
    pub fn from_arrow(batch: arrow::record_batch::RecordBatch) -> Self {
        Self { repr: TableRepr::Arrow(batch) }
    }

    pub fn from_rows(columns: Vec<String>, rows: Vec<Vec<Value>>) -> Self {
        Self { repr: TableRepr::Rows { columns, rows } }
    }

    pub fn column_names(&self) -> Vec<String> {
        match &self.repr {
            TableRepr::Arrow(b) => b.schema().fields().iter().map(|f| f.name().clone()).collect(),
            TableRepr::Rows { columns, .. } => columns.clone(),
        }
    }

    pub fn num_rows(&self) -> usize {
        match &self.repr {
            TableRepr::Arrow(b) => b.num_rows(),
            TableRepr::Rows { rows, .. } => rows.len(),
        }
    }

    pub fn num_columns(&self) -> usize {
        match &self.repr {
            TableRepr::Arrow(b) => b.num_columns(),
            TableRepr::Rows { columns, .. } => columns.len(),
        }
    }

    /// Borrow as a `RecordBatch`. Errors if this is a Rows-backed table.
    /// Slice 1 always succeeds since Rows is not yet constructed.
    pub fn as_arrow(&self) -> Result<&arrow::record_batch::RecordBatch, MError> {
        match &self.repr {
            TableRepr::Arrow(b) => Ok(b),
            TableRepr::Rows { .. } => Err(MError::NotImplemented(
                "operation requires Arrow-backed table (Rows-backed support pending)",
            )),
        }
    }

    /// Owned `RecordBatch` (for sinks that take ownership, e.g. Parquet writer).
    /// Arrow variant: cheap Arc-based clone. Rows variant errors with a clear
    /// message — the Parquet writer (the main sink that calls this) can't
    /// encode heterogeneous cells.
    pub fn try_to_arrow(&self) -> Result<arrow::record_batch::RecordBatch, MError> {
        match &self.repr {
            TableRepr::Arrow(b) => Ok(b.clone()),
            TableRepr::Rows { .. } => Err(MError::Other(
                "table has heterogeneous cells; Arrow encoding requires uniform columns \
                 (coerce mixed cells with Text.From or Table.TransformColumnTypes first)"
                    .into(),
            )),
        }
    }
}

/// Errors raised during evaluation. Per design doc §07 §2, errors propagate
/// automatically through `Result`; `try`/`otherwise` is the only place that
/// observes them and converts back to a Value.
#[derive(Debug, Clone)]
pub enum MError {
    NotImplemented(&'static str),
    NameNotInScope(String),
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    /// User-constructed error value, raised by `error <expr>`. The inner
    /// value is the error record (or pre-lifted record from a text operand).
    Raised(Value),
    /// Generic catch-all replaced by more specific variants as slices surface
    /// real categories.
    Other(String),
}
