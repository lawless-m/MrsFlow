//! Plan IR: the logical relational plan and the scalar sub-IR that sit between
//! the M AST and the connectors. See `mrsflow/10-plan-ir.md`.
//!
//! Built so far:
//!   * the IR data model ([`ir`]),
//!   * S-expression rendering for dump-and-diff ([`mod@sexpr`]),
//!   * lowering from the M AST ([`lower`]),
//!   * logical-optimisation rewrites — conjunction splitting and filter
//!     pushdown ([`optimize`]).
//!
//! All of it is pure and not yet wired into the evaluator, so it changes no
//! observable behaviour. The per-connector fold planner, the DBISAM dialect
//! emitter, projection pruning, and join-pushdown (the last two need a
//! schema-carrying plan) are deliberately later increments.

pub mod ir;
pub mod lower;
pub mod optimize;
mod sexpr;

pub use ir::{
    AggFunc, Aggregation, ArithOp, BoolOp, CmpOp, JoinKind, Lit, ProjectItem, Rel, Scalar, Source,
    SortKey,
};
pub use lower::lower;
pub use optimize::optimize;
