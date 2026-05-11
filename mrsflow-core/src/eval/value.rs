//! Value representation for the M evaluator.
//!
//! See `mrsflow/07-evaluator-design.md` §4 for the full variant list. Variants
//! for kinds not yet needed by a landed slice use placeholder payloads (e.g.
//! `String` for the date types until chrono lands, a tiny `Table` struct until
//! Arrow does). They exist in the enum so evaluator code can pattern-match
//! exhaustively from day one and so the type's shape doesn't change
//! disruptively as later slices land.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::parser::{Expr, Param};

use super::env::{Env, EnvNode};

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Logical(bool),
    Number(f64),
    Text(String),
    /// Placeholder — `chrono::NaiveDate` when the date slice lands.
    Date(String),
    /// Placeholder — `chrono::NaiveDateTime` (and a tz variant) when the date slice lands.
    Datetime(String),
    /// Placeholder — `chrono::Duration` when the date slice lands.
    Duration(String),
    Binary(Vec<u8>),
    List(Vec<Value>),
    /// Records preserve insertion order per spec — `Vec` not `HashMap`.
    Record(Vec<(String, Value)>),
    /// Placeholder — `arrow::RecordBatch` when the Arrow dep lands in eval-7.
    Table(Table),
    Function(Closure),
    Type(TypeRep),
    /// Lazy thunk — forced on first access, memoised thereafter. Central to
    /// M's laziness (per design doc §07 §1).
    Thunk(Rc<RefCell<ThunkState>>),
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub params: Vec<Param>,
    pub body: Box<Expr>,
    pub env: Env,
}

/// Placeholder type-value — the type-system slice (eval-5) replaces this
/// with the full type representation.
#[derive(Debug, Clone)]
pub enum TypeRep {
    Placeholder,
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
#[derive(Debug, Clone)]
pub enum ThunkState {
    Pending { expr: Expr, env: Weak<EnvNode> },
    Forced(Value),
}

/// Placeholder for `Value::Table` until the Arrow dep lands in eval-7.
/// Stored as row-major plus a column-name list — not the eventual
/// representation, just enough for early stdlib stubs to compile.
#[derive(Debug, Clone, Default)]
pub struct Table {
    pub column_names: Vec<String>,
    pub rows: Vec<Vec<Value>>,
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
    /// Generic catch-all replaced by more specific variants as slices surface
    /// real categories.
    Other(String),
}
