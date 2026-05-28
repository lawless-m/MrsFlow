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
//!     composition, and projection pruning.
//!
//! All of it is pure and not yet wired into the evaluator, so it changes no
//! observable behaviour. The per-connector fold planner and the DBISAM dialect
//! emitter are deliberately later increments.

pub mod ir;
pub mod lower;
pub mod optimize;
pub mod schema;
mod sexpr;

pub use ir::{
    AggFunc, Aggregation, ArithOp, BoolOp, CmpOp, JoinKind, Lit, ProjectItem, Rel, Scalar, Source,
    SortKey,
};
pub use lower::lower;
pub use optimize::{optimize, optimize_with_catalog};
pub use schema::{schema_of, Catalog, Schema};
