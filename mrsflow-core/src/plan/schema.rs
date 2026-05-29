//! Schema analysis over the Plan IR.
//!
//! The IR itself stays schema-free (it is a pure logical plan); a node's output
//! schema is *derived* by [`schema_of`], parameterised by a [`Catalog`] that
//! resolves scan leaves to their column lists. This is the planner/binding
//! split: the evaluator or shell supplies a catalog backed by the parquet
//! footer / connector metadata; analyses and tests supply a stub.
//!
//! The analysis is partial. Several operators determine their output schema
//! outright (`Project` with a replacing column list, `Aggregate`), so they
//! resolve even when the leaf catalog cannot. Everything else propagates from
//! its input, and an unresolvable leaf — or an opaque [`Rel::EvalM`], which has
//! no declared output schema yet — yields `None`. Optimisation passes that need
//! a schema simply do nothing where it is unknown, which keeps them sound.

use super::ir::*;

/// An ordered list of output column names. Types are not carried yet — name
/// provenance is all the current passes (pruning, join pushdown) need; typed
/// fields can be added when the dialect emitter's Gate-2 reasoning wants them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    pub columns: Vec<String>,
}

impl Schema {
    pub fn new(columns: Vec<String>) -> Self {
        Self { columns }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.columns.iter().any(|c| c == name)
    }

    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }
}

/// Resolves a [`Source`] (a scan leaf) to its output schema. Returns `None` for
/// sources it cannot resolve, which propagates as an unknown schema.
pub trait Catalog {
    fn schema_of_source(&self, source: &Source) -> Option<Schema>;
}

/// Derive the output schema of a relational plan, or `None` if it cannot be
/// determined (an unresolvable leaf or an opaque `EvalM` anywhere a passthrough
/// node depends on it).
pub fn schema_of(rel: &Rel, catalog: &dyn Catalog) -> Option<Schema> {
    match rel {
        Rel::Scan(src) => catalog.schema_of_source(src),

        // Passthrough operators: output schema equals input schema.
        Rel::Filter { input, .. }
        | Rel::Sort { input, .. }
        | Rel::Limit { input, .. }
        | Rel::Distinct { input, .. } => schema_of(input, catalog),

        Rel::Project { star, items, input } => {
            let item_names = items.iter().map(|i| i.name.clone());
            if *star {
                // AddColumn-shape: input columns, then the added names appended.
                let mut cols = schema_of(input, catalog)?.columns;
                for n in item_names {
                    if !cols.contains(&n) {
                        cols.push(n);
                    }
                }
                Some(Schema::new(cols))
            } else {
                // SelectColumns-shape: the item list fully determines the output.
                Some(Schema::new(item_names.collect()))
            }
        }

        Rel::Aggregate { keys, aggs, .. } => {
            let mut cols: Vec<String> = keys
                .iter()
                .map(|k| match k {
                    Scalar::Col(n) => n.clone(),
                    Scalar::QualifiedCol { name, .. } => name.clone(),
                    _ => String::new(),
                })
                .collect();
            cols.extend(aggs.iter().map(|a| a.name.clone()));
            Some(Schema::new(cols))
        }

        Rel::Join { left, right, .. } => {
            // Logical join output: the left columns followed by the right
            // columns. (The nested-column shape the M source carried is
            // collapsed; flattening is a later refinement.)
            let mut cols = schema_of(left, catalog)?.columns;
            cols.extend(schema_of(right, catalog)?.columns);
            Some(Schema::new(cols))
        }

        // Opaque: no declared output schema.
        Rel::EvalM { .. } => None,
    }
}
