//! Plan IR: the logical relational plan and the scalar sub-IR that sit between
//! the M AST and the connectors. See `mrsflow/10-plan-ir.md`.
//!
//! This is the foundation increment: the IR data model ([`ir`]), S-expression
//! rendering for dump-and-diff ([`mod@sexpr`]), and lowering from the M AST
//! ([`lower`]). It is pure and not yet wired into the evaluator, so it changes
//! no observable behaviour. The logical-optimisation passes, the per-connector
//! fold planner, and the DBISAM dialect emitter are deliberately later
//! increments.

pub mod ir;
pub mod lower;
mod sexpr;

pub use ir::{
    AggFunc, Aggregation, ArithOp, BoolOp, CmpOp, JoinKind, Lit, ProjectItem, Rel, Scalar, Source,
    SortKey,
};
pub use lower::lower;
