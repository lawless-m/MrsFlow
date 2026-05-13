//! Value representation for the M evaluator.
//!
//! See `mrsflow/07-evaluator-design.md` §4 for the full variant list. Variants
//! for kinds not yet needed by a landed slice use placeholder payloads (e.g.
//! `String` for the date types until chrono lands, a tiny `Table` struct until
//! Arrow does). They exist in the enum so evaluator code can pattern-match
//! exhaustively from day one and so the type's shape doesn't change
//! disruptively as later slices land.

use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt;
use std::rc::{Rc, Weak};
use std::sync::Arc;

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
    /// Parquet bytes + projection state, undecoded. Forced into `Arrow`
    /// on any op that needs row data (see `Table::force`). Projection-
    /// aware ops (`Table.SelectColumns`, etc.) narrow `projection`
    /// without forcing, so columns the M source never touches are
    /// never decoded. See `mrsflow/09-lazy-tables.md`.
    LazyParquet(LazyParquetState),
    /// Deferred result of `Table.NestedJoin`: left table + right handle +
    /// per-left-row indices into right. The nested-column cell at each
    /// row is conceptually a Table containing the matched right rows,
    /// but those Tables aren't materialised until forced — letting
    /// `Table.ExpandTableColumn` pull only the requested columns from
    /// the right handle. RT-preserving by construction: forcing yields
    /// the byte-identical Rows-backed result the eager path would have
    /// produced. See `mrsflow/09-lazy-tables.md` §4.
    JoinView(JoinViewState),
    /// Deferred result of `Table.ExpandTableColumn` over a `JoinView`.
    /// Holds (lazy) left + projections of left and right + per-outer-row
    /// match indices. Column count, row count and column names are
    /// available without forcing; cell access forces the same way as
    /// the eager `expand_table_column` would have produced. The big win:
    /// `Table.RowCount` / chained `SelectColumns` / `RemoveColumns` /
    /// `Table.NestedJoin` can operate on this view without materialising
    /// either side's bulk columns. See `mrsflow/09-lazy-tables.md`
    /// (Stage A.5).
    ExpandView(ExpandViewState),
}

/// State for a `TableRepr::ExpandView`. Constructed by
/// `Table.ExpandTableColumn` over a `JoinView`; preserves enough
/// information for `SelectColumns`/`RemoveColumns`/`NestedJoin` chains
/// to keep operating lazily, deferring left- and right-side decode
/// until something genuinely needs a cell.
#[derive(Debug, Clone)]
pub struct ExpandViewState {
    /// Source left table — possibly lazy. Cell access for a left
    /// column at outer row `i` reads `left` at row `i`.
    pub left: Arc<Table>,
    /// Indices of left columns to include in the output, in output
    /// order. Positions in `left.column_names()` (which may itself be
    /// a projection if `left` is a `LazyParquet` with a narrowed mask).
    pub left_projection: Vec<usize>,
    /// Source right table — possibly lazy.
    pub right: Arc<Table>,
    /// Indices of right columns to include in the output, in the
    /// order in which the expand's `column_names` arg listed them.
    pub right_projection: Vec<usize>,
    /// Output names for the right columns — parallel to
    /// `right_projection`, lets `Table.ExpandTableColumn` rename
    /// the expanded columns at view-construction time.
    pub right_output_names: Vec<String>,
    /// For each row of `left`, indices of matched rows in `right`.
    /// Empty matches drop the outer row from the output (matches the
    /// eager expand semantics: empty inner table → 0 output rows).
    pub matches: Vec<Vec<u32>>,
}

/// State for a `TableRepr::JoinView`. Constructed by `Table.NestedJoin`
/// when the right side is a `LazyParquet` (or `Arrow`); preserves enough
/// information for downstream `Table.ExpandTableColumn` to pull only
/// the requested right-side columns. Forces into a Rows-backed table.
#[derive(Debug, Clone)]
pub struct JoinViewState {
    /// Left table — stays lazy where lazy. NestedJoin decodes only the
    /// left key column to build matches; the full left side decodes
    /// later only when something forces (e.g. a force-on-entry stdlib
    /// function, or `materialise_join_view`). `Arc` to share with
    /// downstream lazy nodes (ExpandView) cheaply.
    pub left: Arc<Table>,
    /// The right-side table. May itself be `LazyParquet` (in which case
    /// only the columns expand pulls are decoded) or an eager variant.
    pub right: Arc<Table>,
    /// The output column name for the nested column.
    pub new_column_name: String,
    /// For each row in `left`, the indices of matched rows in `right`.
    /// Computed eagerly by `Table.NestedJoin`'s hash-join pass, since
    /// computing matches requires decoding just the key columns of
    /// each side.
    pub matches: Vec<Vec<u32>>,
    /// Indices of `right` rows that no left row matched. Used by the
    /// RightOuter / FullOuter / RightAnti materialisation paths to
    /// emit null-left rows for unmatched right entries. Computed once
    /// in `nested_join` (same pass as `matches`).
    pub unmatched_right: Vec<u32>,
    /// Join kind passed to NestedJoin: 0=Inner, 1=LeftOuter,
    /// 2=RightOuter, 3=FullOuter, 4=LeftAnti, 5=RightAnti.
    pub join_kind: i32,
}

/// State for a `TableRepr::LazyParquet`. Constructed by
/// `Table::lazy_parquet`; mutated only by projection-aware ops via
/// clone-and-replace.
#[derive(Debug, Clone)]
pub struct LazyParquetState {
    /// Parquet file bytes. `Arc` so cloning a Table doesn't duplicate
    /// up to 200MB of buffer.
    pub bytes: Arc<bytes::Bytes>,
    /// Schema as read from the parquet footer at construction time.
    /// Indices into `schema.fields()` are stable identifiers used by
    /// `projection`.
    pub schema: arrow::datatypes::SchemaRef,
    /// Column indices into `schema.fields()`, in output order.
    /// Initially `(0..schema.fields().len()).collect()`. Narrowed by
    /// `Table.SelectColumns` / `RemoveColumns` / `ReorderColumns`.
    pub projection: Vec<usize>,
    /// Total row count summed across row groups, cached from the
    /// footer at construction. Lets `Table.RowCount` return without
    /// decoding any data.
    pub num_rows: usize,
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

    /// Construct a lazy parquet-backed table. Reads only the footer to
    /// obtain the schema and row count; row data stays in `bytes` until
    /// a force call decodes it. See `mrsflow/09-lazy-tables.md` §3.
    pub fn lazy_parquet(bytes: bytes::Bytes) -> Result<Self, MError> {
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
        let builder = ParquetRecordBatchReaderBuilder::try_new(bytes.clone())
            .map_err(|e| MError::Other(format!("LazyParquet: footer read failed: {e}")))?;
        let schema = builder.schema().clone();
        let num_rows: usize = builder
            .metadata()
            .row_groups()
            .iter()
            .map(|rg| rg.num_rows() as usize)
            .sum();
        let projection: Vec<usize> = (0..schema.fields().len()).collect();
        Ok(Self {
            repr: TableRepr::LazyParquet(LazyParquetState {
                bytes: Arc::new(bytes),
                schema,
                projection,
                num_rows,
            }),
        })
    }

    pub fn column_names(&self) -> Vec<String> {
        match &self.repr {
            TableRepr::Arrow(b) => b.schema().fields().iter().map(|f| f.name().clone()).collect(),
            TableRepr::Rows { columns, .. } => columns.clone(),
            TableRepr::LazyParquet(s) => s
                .projection
                .iter()
                .map(|&i| s.schema.field(i).name().clone())
                .collect(),
            TableRepr::JoinView(jv) => {
                // left columns + the new nested column at the end.
                let mut names = jv.left.column_names();
                names.push(jv.new_column_name.clone());
                names
            }
            TableRepr::ExpandView(ev) => {
                let left_names = ev.left.column_names();
                let mut names: Vec<String> = ev
                    .left_projection
                    .iter()
                    .map(|&i| left_names[i].clone())
                    .collect();
                names.extend(ev.right_output_names.iter().cloned());
                names
            }
        }
    }

    pub fn num_rows(&self) -> usize {
        match &self.repr {
            TableRepr::Arrow(b) => b.num_rows(),
            TableRepr::Rows { rows, .. } => rows.len(),
            TableRepr::LazyParquet(s) => s.num_rows,
            TableRepr::JoinView(jv) => {
                // Row count per join kind:
                //   Inner       = left rows with ≥1 match
                //   LeftOuter   = all left rows
                //   RightOuter  = sum(matches[i].len()) + unmatched_right
                //                 (one row per (left, right) match pair,
                //                 plus one null-left row per unmatched right)
                //   FullOuter   = LeftOuter rows + unmatched_right
                //   LeftAnti    = left rows with 0 matches
                //   RightAnti   = unmatched_right
                match jv.join_kind {
                    0 => jv.matches.iter().filter(|m| !m.is_empty()).count(),
                    1 => jv.left.num_rows(),
                    2 => {
                        jv.matches.iter().map(|m| m.len()).sum::<usize>()
                            + jv.unmatched_right.len()
                    }
                    3 => jv.left.num_rows() + jv.unmatched_right.len(),
                    4 => jv.matches.iter().filter(|m| m.is_empty()).count(),
                    5 => jv.unmatched_right.len(),
                    _ => jv.left.num_rows(),
                }
            }
            TableRepr::ExpandView(ev) => {
                // Expanded rows = sum over matches[i].len(). Empty
                // matches drop their outer row, matching eager expand.
                ev.matches.iter().map(|m| m.len()).sum()
            }
        }
    }

    pub fn num_columns(&self) -> usize {
        match &self.repr {
            TableRepr::Arrow(b) => b.num_columns(),
            TableRepr::Rows { columns, .. } => columns.len(),
            TableRepr::LazyParquet(s) => s.projection.len(),
            TableRepr::JoinView(jv) => jv.left.num_columns() + 1,
            TableRepr::ExpandView(ev) => {
                ev.left_projection.len() + ev.right_output_names.len()
            }
        }
    }

    /// Borrow as a `RecordBatch`. Errors if this is a Rows-backed table.
    /// For LazyParquet tables, callers should force first via `force()`;
    /// this method only succeeds on already-Arrow tables.
    pub fn as_arrow(&self) -> Result<&arrow::record_batch::RecordBatch, MError> {
        match &self.repr {
            TableRepr::Arrow(b) => Ok(b),
            TableRepr::Rows { .. } => Err(MError::NotImplemented(
                "operation requires Arrow-backed table (Rows-backed support pending)",
            )),
            TableRepr::LazyParquet(_)
            | TableRepr::JoinView(_)
            | TableRepr::ExpandView(_) => Err(MError::Other(
                "internal: as_arrow() called on lazy table without forcing first \
                 — use Table::force() or expect_table()".into(),
            )),
        }
    }

    /// Owned `RecordBatch` (for sinks that take ownership, e.g. Parquet writer).
    /// Arrow variant: cheap Arc-based clone. Rows variant errors with a clear
    /// message — the Parquet writer (the main sink that calls this) can't
    /// encode heterogeneous cells. LazyParquet forces with current projection
    /// before returning. JoinView forces to Rows then fails (nested-Table
    /// cells aren't Arrow-encodable, same as Rows variant).
    pub fn try_to_arrow(&self) -> Result<arrow::record_batch::RecordBatch, MError> {
        match &self.repr {
            TableRepr::Arrow(b) => Ok(b.clone()),
            TableRepr::Rows { .. } => Err(MError::Other(
                "table has heterogeneous cells; Arrow encoding requires uniform columns \
                 (coerce mixed cells with Text.From or Table.TransformColumnTypes first)"
                    .into(),
            )),
            TableRepr::LazyParquet(s) => decode_lazy_parquet(s),
            TableRepr::JoinView(_) => Err(MError::Other(
                "Table.NestedJoin result contains nested Table-valued cells; \
                 Arrow encoding requires uniform columns. Use \
                 Table.ExpandTableColumn first to flatten."
                    .into(),
            )),
            TableRepr::ExpandView(_) => {
                // Force into a Rows-backed Table first, then try the
                // Rows→Arrow path. Most ExpandView results have mixed
                // typed columns and will fail at the Rows branch above
                // with a clear message; that's the same outcome as the
                // eager `expand_table_column` would have produced.
                let forced = self.force()?;
                forced.try_to_arrow()
            }
        }
    }

    /// Materialise a lazy table into its eager form. For already-Arrow or
    /// Rows tables, returns the original by borrow (no copy). For
    /// LazyParquet, decodes the projected columns. For JoinView, walks
    /// `matches` against the right side and constructs the nested-Table
    /// cells the eager NestedJoin would have produced. See
    /// `mrsflow/09-lazy-tables.md` §5.
    pub fn force(&self) -> Result<Cow<'_, Self>, MError> {
        match &self.repr {
            TableRepr::Arrow(_) | TableRepr::Rows { .. } => Ok(Cow::Borrowed(self)),
            TableRepr::LazyParquet(s) => Ok(Cow::Owned(Self::from_arrow(decode_lazy_parquet(s)?))),
            TableRepr::JoinView(jv) => Ok(Cow::Owned(materialise_join_view(jv)?)),
            TableRepr::ExpandView(ev) => Ok(Cow::Owned(materialise_expand_view(ev)?)),
        }
    }
}

/// Narrow `t` to just the columns at positions `cols` (in output order),
/// without forcing if the input is `LazyParquet` (rewrites the mask),
/// `Arrow` (Arc-cheap column select), or `Rows` (per-row index pick).
/// For `JoinView`/`ExpandView` the input is forced first and then
/// narrowed — recursive narrowing through chained-lazy sources is a
/// future improvement. The common corpus path
/// (LazyParquet → JoinView → ExpandView → narrowed) only ever hits the
/// LazyParquet branch when materialising.
fn narrow_for_force(t: &Table, cols: &[usize]) -> Result<Table, MError> {
    let names = t.column_names();
    let new_names: Vec<String> = cols.iter().map(|&i| names[i].clone()).collect();
    match &t.repr {
        TableRepr::LazyParquet(state) => {
            let new_projection: Vec<usize> =
                cols.iter().map(|&i| state.projection[i]).collect();
            Ok(Table {
                repr: TableRepr::LazyParquet(LazyParquetState {
                    bytes: state.bytes.clone(),
                    schema: state.schema.clone(),
                    projection: new_projection,
                    num_rows: state.num_rows,
                }),
            })
        }
        TableRepr::Arrow(batch) => {
            let schema = batch.schema();
            let new_fields: Vec<arrow::datatypes::Field> = cols
                .iter()
                .map(|&i| schema.field(i).clone())
                .collect();
            let new_schema = Arc::new(arrow::datatypes::Schema::new(new_fields));
            let new_columns: Vec<arrow::array::ArrayRef> =
                cols.iter().map(|&i| batch.column(i).clone()).collect();
            let new_batch = arrow::record_batch::RecordBatch::try_new(new_schema, new_columns)
                .map_err(|e| MError::Other(format!("narrow_for_force: {e}")))?;
            Ok(Table::from_arrow(new_batch))
        }
        TableRepr::Rows { rows, .. } => {
            let new_rows: Vec<Vec<Value>> = rows
                .iter()
                .map(|row| cols.iter().map(|&i| row[i].clone()).collect())
                .collect();
            Ok(Table::from_rows(new_names, new_rows))
        }
        TableRepr::JoinView(_) | TableRepr::ExpandView(_) => {
            // Force then narrow. Sub-optimal for nested lazies but
            // correct; the chained-lazy decode is the deeper
            // optimisation that isn't done here.
            let forced = t.force()?;
            narrow_for_force(&forced, cols)
        }
    }
}

/// Materialise an `ExpandView` into a `Rows`-backed Table — byte-
/// identical to what the eager `Table.NestedJoin` + `Table.ExpandTableColumn`
/// path would have produced. Narrows left/right to just the projected
/// columns BEFORE forcing, so that `LazyParquet` sources decode only
/// the columns this ExpandView actually exposes (rather than their full
/// current projection).
fn materialise_expand_view(ev: &ExpandViewState) -> Result<Table, MError> {
    // Narrow each side to just its projection before forcing — the
    // critical optimisation. For a 40-col LazyParquet left where this
    // ExpandView only exposes 10 columns, we decode 10 not 40.
    let left_narrowed = narrow_for_force(&ev.left, &ev.left_projection)?;
    let right_narrowed = narrow_for_force(&ev.right, &ev.right_projection)?;
    let left_forced = left_narrowed.force()?;
    let right_forced = right_narrowed.force()?;
    let left_table: &Table = &left_forced;
    let right_table: &Table = &right_forced;

    // After narrowing, left has columns 0..ev.left_projection.len() and
    // right has columns 0..ev.right_projection.len() — no longer indexed
    // by the original projection.
    let n_left = ev.left_projection.len();
    let n_right = ev.right_projection.len();

    let read_row = |t: &Table, row: usize, n: usize| -> Result<Vec<Value>, MError> {
        match &t.repr {
            TableRepr::Rows { rows, .. } => Ok(rows[row].clone()),
            TableRepr::Arrow(batch) => {
                let mut out = Vec::with_capacity(n);
                for c in 0..n {
                    out.push(arrow_cell_to_value(batch, c, row)?);
                }
                Ok(out)
            }
            _ => Err(MError::Other(
                "materialise_expand_view: unexpected non-eager variant after force".into(),
            )),
        }
    };

    let left_names = left_table.column_names();
    let mut out_names: Vec<String> = left_names;
    out_names.extend(ev.right_output_names.iter().cloned());

    let total: usize = ev.matches.iter().map(|m| m.len()).sum();
    let mut out_rows: Vec<Vec<Value>> = Vec::with_capacity(total);

    for (outer_idx, match_indices) in ev.matches.iter().enumerate() {
        if match_indices.is_empty() {
            continue;
        }
        let left_cells = read_row(left_table, outer_idx, n_left)?;
        for &right_idx in match_indices {
            let mut row = left_cells.clone();
            let right_cells = read_row(right_table, right_idx as usize, n_right)?;
            row.extend(right_cells);
            out_rows.push(row);
        }
    }

    Ok(Table::from_rows(out_names, out_rows))
}

/// Materialise a `JoinView` into a `Rows`-backed Table — byte-identical
/// to what the eager `Table.NestedJoin` path would have produced. Forces
/// both sides, walks `matches`, builds nested Table-valued cells per
/// row. Inner-join drops unmatched left rows; LeftOuter keeps them with
/// empty nested tables.
fn materialise_join_view(jv: &JoinViewState) -> Result<Table, MError> {
    let left_forced = jv.left.force()?;
    let right_forced = jv.right.force()?;
    let left_table: &Table = &left_forced;
    let right_table: &Table = &right_forced;

    let left_names = left_table.column_names();
    let right_names = right_table.column_names();

    // Helper to read a row from an Arrow/Rows table into a Vec<Value>.
    // We can't call stdlib::cell_to_value from here without a cycle, so
    // duplicate the small dispatch inline. JoinView's left/right are
    // always Arrow or Rows after force (LazyParquet decodes into Arrow).
    let read_row = |t: &Table, row: usize| -> Result<Vec<Value>, MError> {
        match &t.repr {
            TableRepr::Rows { rows, .. } => Ok(rows[row].clone()),
            TableRepr::Arrow(batch) => {
                let mut out = Vec::with_capacity(batch.num_columns());
                for col in 0..batch.num_columns() {
                    out.push(arrow_cell_to_value(batch, col, row)?);
                }
                Ok(out)
            }
            _ => Err(MError::Other(
                "materialise_join_view: unexpected non-eager variant after force".into(),
            )),
        }
    };

    let mut out_names: Vec<String> = left_names.clone();
    out_names.push(jv.new_column_name.clone());

    // Capacity estimate per kind. Avoids pathological reallocations on
    // large outputs; cheap to slightly over-estimate.
    let cap = match jv.join_kind {
        0 => jv.matches.iter().filter(|m| !m.is_empty()).count(),
        1 => jv.left.num_rows(),
        2 => jv.matches.iter().map(|m| m.len()).sum::<usize>() + jv.unmatched_right.len(),
        3 => jv.left.num_rows() + jv.unmatched_right.len(),
        4 => jv.matches.iter().filter(|m| m.is_empty()).count(),
        5 => jv.unmatched_right.len(),
        _ => jv.left.num_rows(),
    };
    let mut out_rows: Vec<Vec<Value>> = Vec::with_capacity(cap);

    // null_left_row: reused for unmatched-right rows under
    // RightOuter / FullOuter / RightAnti. Power Query convention: the
    // left columns become null when the row comes from the right side
    // with no match. Build once.
    let null_left_row: Vec<Value> = vec![Value::Null; left_names.len()];

    match jv.join_kind {
        // Inner: emit one row per left row that has ≥1 match.
        // Nested column = Table of matched right rows.
        0 => {
            for (left_idx, match_indices) in jv.matches.iter().enumerate() {
                if match_indices.is_empty() {
                    continue;
                }
                let nested_rows: Vec<Vec<Value>> = match_indices
                    .iter()
                    .map(|&i| read_row(right_table, i as usize))
                    .collect::<Result<_, _>>()?;
                let nested_table = Table::from_rows(right_names.clone(), nested_rows);
                let mut row = read_row(left_table, left_idx)?;
                row.push(Value::Table(nested_table));
                out_rows.push(row);
            }
        }
        // LeftOuter: emit every left row, with matched-right Table
        // (possibly empty) in the nested column.
        1 => {
            for (left_idx, match_indices) in jv.matches.iter().enumerate() {
                let nested_rows: Vec<Vec<Value>> = match_indices
                    .iter()
                    .map(|&i| read_row(right_table, i as usize))
                    .collect::<Result<_, _>>()?;
                let nested_table = Table::from_rows(right_names.clone(), nested_rows);
                let mut row = read_row(left_table, left_idx)?;
                row.push(Value::Table(nested_table));
                out_rows.push(row);
            }
        }
        // RightOuter: iterate (left, right) match pairs — one row per
        // pair, left columns from the matching left row, nested
        // contains just that one right row. Then append one row per
        // unmatched right with null left columns.
        2 => {
            for (left_idx, match_indices) in jv.matches.iter().enumerate() {
                for &right_idx in match_indices {
                    let mut row = read_row(left_table, left_idx)?;
                    let nested_table = Table::from_rows(
                        right_names.clone(),
                        vec![read_row(right_table, right_idx as usize)?],
                    );
                    row.push(Value::Table(nested_table));
                    out_rows.push(row);
                }
            }
            for &right_idx in &jv.unmatched_right {
                let mut row = null_left_row.clone();
                let nested_table = Table::from_rows(
                    right_names.clone(),
                    vec![read_row(right_table, right_idx as usize)?],
                );
                row.push(Value::Table(nested_table));
                out_rows.push(row);
            }
        }
        // FullOuter: LeftOuter rows (every left row, possibly empty
        // nested) plus one null-left row per unmatched right.
        3 => {
            for (left_idx, match_indices) in jv.matches.iter().enumerate() {
                let nested_rows: Vec<Vec<Value>> = match_indices
                    .iter()
                    .map(|&i| read_row(right_table, i as usize))
                    .collect::<Result<_, _>>()?;
                let nested_table = Table::from_rows(right_names.clone(), nested_rows);
                let mut row = read_row(left_table, left_idx)?;
                row.push(Value::Table(nested_table));
                out_rows.push(row);
            }
            for &right_idx in &jv.unmatched_right {
                let mut row = null_left_row.clone();
                let nested_table = Table::from_rows(
                    right_names.clone(),
                    vec![read_row(right_table, right_idx as usize)?],
                );
                row.push(Value::Table(nested_table));
                out_rows.push(row);
            }
        }
        // LeftAnti: emit only left rows with NO match. Nested column
        // is always an empty Table.
        4 => {
            for (left_idx, match_indices) in jv.matches.iter().enumerate() {
                if !match_indices.is_empty() {
                    continue;
                }
                let nested_table =
                    Table::from_rows(right_names.clone(), Vec::new());
                let mut row = read_row(left_table, left_idx)?;
                row.push(Value::Table(nested_table));
                out_rows.push(row);
            }
        }
        // RightAnti: emit only unmatched-right rows, with null left
        // columns and nested table containing the unmatched right row.
        5 => {
            for &right_idx in &jv.unmatched_right {
                let mut row = null_left_row.clone();
                let nested_table = Table::from_rows(
                    right_names.clone(),
                    vec![read_row(right_table, right_idx as usize)?],
                );
                row.push(Value::Table(nested_table));
                out_rows.push(row);
            }
        }
        _ => {
            return Err(MError::Other(format!(
                "Table.NestedJoin: invalid joinKind {} (must be 0–5)",
                jv.join_kind
            )));
        }
    }

    Ok(Table::from_rows(out_names, out_rows))
}

/// Local minimal copy of stdlib's `cell_to_value` covering the types
/// the NestedJoin / ExpandTableColumn path actually surfaces. Used by
/// `materialise_join_view` to avoid an upward dep on the stdlib module.
fn arrow_cell_to_value(
    batch: &arrow::record_batch::RecordBatch,
    col: usize,
    row: usize,
) -> Result<Value, MError> {
    use arrow::array::*;
    use arrow::datatypes::{DataType, TimeUnit};
    let array = batch.column(col);
    if array.is_null(row) {
        return Ok(Value::Null);
    }
    match array.data_type() {
        DataType::Float64 => Ok(Value::Number(
            array.as_any().downcast_ref::<Float64Array>().expect("Float64").value(row),
        )),
        DataType::Float32 => Ok(Value::Number(
            array.as_any().downcast_ref::<Float32Array>().expect("Float32").value(row) as f64,
        )),
        DataType::Int8 => Ok(Value::Number(
            array.as_any().downcast_ref::<Int8Array>().expect("Int8").value(row) as f64,
        )),
        DataType::Int16 => Ok(Value::Number(
            array.as_any().downcast_ref::<Int16Array>().expect("Int16").value(row) as f64,
        )),
        DataType::Int32 => Ok(Value::Number(
            array.as_any().downcast_ref::<Int32Array>().expect("Int32").value(row) as f64,
        )),
        DataType::Int64 => Ok(Value::Number(
            array.as_any().downcast_ref::<Int64Array>().expect("Int64").value(row) as f64,
        )),
        DataType::UInt8 => Ok(Value::Number(
            array.as_any().downcast_ref::<UInt8Array>().expect("UInt8").value(row) as f64,
        )),
        DataType::UInt16 => Ok(Value::Number(
            array.as_any().downcast_ref::<UInt16Array>().expect("UInt16").value(row) as f64,
        )),
        DataType::UInt32 => Ok(Value::Number(
            array.as_any().downcast_ref::<UInt32Array>().expect("UInt32").value(row) as f64,
        )),
        DataType::UInt64 => Ok(Value::Number(
            array.as_any().downcast_ref::<UInt64Array>().expect("UInt64").value(row) as f64,
        )),
        DataType::Utf8 => Ok(Value::Text(
            array.as_any().downcast_ref::<StringArray>().expect("Utf8").value(row).to_string(),
        )),
        DataType::Boolean => Ok(Value::Logical(
            array.as_any().downcast_ref::<BooleanArray>().expect("Boolean").value(row),
        )),
        DataType::Date32 => {
            let days = array.as_any().downcast_ref::<Date32Array>().expect("Date32").value(row);
            let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            let d = epoch
                .checked_add_signed(chrono::Duration::days(days as i64))
                .ok_or_else(|| MError::Other(format!("Date32 out of range: {days} days")))?;
            Ok(Value::Date(d))
        }
        // All Timestamp variants collapse to Value::Datetime; see the
        // matching arm in `stdlib/table.rs` for rationale.
        DataType::Timestamp(unit, _tz) => {
            let micros: i64 = match unit {
                TimeUnit::Second => array
                    .as_any()
                    .downcast_ref::<TimestampSecondArray>()
                    .expect("TimestampSecond")
                    .value(row)
                    .saturating_mul(1_000_000),
                TimeUnit::Millisecond => array
                    .as_any()
                    .downcast_ref::<TimestampMillisecondArray>()
                    .expect("TimestampMillisecond")
                    .value(row)
                    .saturating_mul(1_000),
                TimeUnit::Microsecond => array
                    .as_any()
                    .downcast_ref::<TimestampMicrosecondArray>()
                    .expect("TimestampMicrosecond")
                    .value(row),
                TimeUnit::Nanosecond => array
                    .as_any()
                    .downcast_ref::<TimestampNanosecondArray>()
                    .expect("TimestampNanosecond")
                    .value(row)
                    / 1_000,
            };
            let dt = chrono::DateTime::from_timestamp_micros(micros)
                .ok_or_else(|| MError::Other(format!("Timestamp out of range: {micros} us")))?
                .naive_utc();
            Ok(Value::Datetime(dt))
        }
        DataType::Date64 => {
            let millis = array
                .as_any()
                .downcast_ref::<Date64Array>()
                .expect("Date64")
                .value(row);
            let dt = chrono::DateTime::from_timestamp_millis(millis)
                .ok_or_else(|| MError::Other(format!("Date64 out of range: {millis} ms")))?
                .date_naive();
            Ok(Value::Date(dt))
        }
        DataType::Null => Ok(Value::Null),
        _ => Err(MError::NotImplemented("unsupported cell type")),
    }
}

/// Decode a `LazyParquetState` into a `RecordBatch`, reading only the
/// columns named by `state.projection`.
fn decode_lazy_parquet(
    state: &LazyParquetState,
) -> Result<arrow::record_batch::RecordBatch, MError> {
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
    use parquet::arrow::ProjectionMask;
    let builder =
        ParquetRecordBatchReaderBuilder::try_new(state.bytes.as_ref().clone())
            .map_err(|e| MError::Other(format!("LazyParquet decode: {e}")))?;
    let mask = ProjectionMask::roots(
        builder.parquet_schema(),
        state.projection.iter().copied(),
    );
    let reader = builder
        .with_projection(mask)
        .build()
        .map_err(|e| MError::Other(format!("LazyParquet decode: {e}")))?;
    let batches: Vec<arrow::record_batch::RecordBatch> = reader
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| MError::Other(format!("LazyParquet decode: {e}")))?;
    match batches.len() {
        0 => Ok(arrow::record_batch::RecordBatch::new_empty(Arc::new(
            arrow::datatypes::Schema::empty(),
        ))),
        1 => Ok(batches.into_iter().next().expect("len == 1")),
        _ => arrow::compute::concat_batches(&batches[0].schema(), &batches)
            .map_err(|e| MError::Other(format!("LazyParquet decode concat: {e}"))),
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
