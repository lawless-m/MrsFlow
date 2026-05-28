//! Plan IR: the logical relational plan and the scalar sub-IR that sit between
//! the M AST and the connectors. See `mrsflow/10-plan-ir.md`.
//!
//! Built so far:
//!   * the IR data model ([`ir`]),
//!   * S-expression rendering for dump-and-diff ([`mod@sexpr`]),
//!   * lowering from the M AST ([`lower`]),
//!   * schema analysis over the plan, parameterised by a [`Catalog`]
//!     ([`schema`]),
//!   * logical-optimisation rewrites ([`optimize`]) — conjunction splitting,
//!     filter pushdown (incl. join pushdown via column provenance), project
//!     composition, and projection pruning,
//!   * connector SQL emission + the fold split ([`fold`]) — the DBISAM dialect
//!     emitter (Gate 1: the emitter's success *is* the fold predicate) and the
//!     execution split into pushed SQL plus an in-memory residual.
//!
//! All of it is pure and not yet wired into the evaluator, so it changes no
//! observable behaviour. Wiring the fold into the connectors, and the
//! differential gate (Gate 2) that must pass before folding against a live
//! source, are deliberately later increments.

pub mod fold;
pub mod ir;
pub mod lower;
pub mod optimize;
pub mod schema;
mod sexpr;

pub use fold::{emit, fold, Dbisam, Dialect, FoldResult, SelectParts, Unfoldable, FOLDED};
pub use ir::{
    AggFunc, Aggregation, ArithOp, BoolOp, CmpOp, JoinKind, Lit, ProjectItem, Rel, Scalar, Source,
    SortKey,
};
pub use lower::lower;
pub use optimize::{optimize, optimize_with_catalog};
pub use schema::{schema_of, Catalog, Schema};
