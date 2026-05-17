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

// Profiling counters — bumped from the custom Clone impl below to find
// hot paths. Read via `value::PROFILE.snapshot()` from the CLI.
#[cfg(feature = "profile-clones")]
pub mod profile {
    use std::sync::atomic::{AtomicU64, Ordering};
    pub static LIST_CLONES: AtomicU64 = AtomicU64::new(0);
    pub static LIST_TOTAL_LEN: AtomicU64 = AtomicU64::new(0);
    // Size buckets: 0, 1, 2-3, 4-7, 8-15, ..., 16384-32767, 32768+
    pub static LIST_CLONE_BUCKETS: [AtomicU64; 16] = [
        AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
        AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
        AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
        AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
    ];
    pub static LIST_CLONE_MAX_LEN: AtomicU64 = AtomicU64::new(0);
    pub static RECORD_CLONES: AtomicU64 = AtomicU64::new(0);
    pub static TEXT_CLONES: AtomicU64 = AtomicU64::new(0);
    pub static BINARY_CLONES: AtomicU64 = AtomicU64::new(0);
    pub static BINARY_TOTAL_BYTES: AtomicU64 = AtomicU64::new(0);
    // env::lookup counters — every lookup increments ENV_LOOKUPS;
    // ENV_LIST_LOOKUPS only when the bound value happens to be a List
    // (so subtracting that from LIST_CLONES tells us how many list
    // clones came from elsewhere — stdlib internals, etc).
    pub static ENV_LOOKUPS: AtomicU64 = AtomicU64::new(0);
    pub static ENV_LIST_LOOKUPS: AtomicU64 = AtomicU64::new(0);
    pub static ENV_LIST_LOOKUP_TOTAL_LEN: AtomicU64 = AtomicU64::new(0);
    // force(thunk) where thunk is already Forced — currently clones
    // the memoised value. If FORCE_LIST_HITS is the bulk of LIST_CLONES,
    // sharing the forced value via Rc is the fix.
    pub static FORCE_HITS: AtomicU64 = AtomicU64::new(0);
    pub static FORCE_LIST_HITS: AtomicU64 = AtomicU64::new(0);
    pub fn bump_list(len: usize) {
        LIST_CLONES.fetch_add(1, Ordering::Relaxed);
        LIST_TOTAL_LEN.fetch_add(len as u64, Ordering::Relaxed);
        // Bucket by log2(len) — index 0 = len 0, index 1 = len 1,
        // index 2 = len 2-3, ..., index 15 = len 16384+.
        let bucket = if len == 0 { 0 }
            else if len == 1 { 1 }
            else {
                let lg = 64 - (len as u64).leading_zeros() as usize; // ceil(log2)+1
                lg.min(15)
            };
        LIST_CLONE_BUCKETS[bucket].fetch_add(1, Ordering::Relaxed);
        let mut prev = LIST_CLONE_MAX_LEN.load(Ordering::Relaxed);
        while (len as u64) > prev {
            match LIST_CLONE_MAX_LEN.compare_exchange_weak(
                prev, len as u64, Ordering::Relaxed, Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => prev = p,
            }
        }
    }
    pub fn bump_record() { RECORD_CLONES.fetch_add(1, Ordering::Relaxed); }
    pub fn bump_text() { TEXT_CLONES.fetch_add(1, Ordering::Relaxed); }
    pub fn bump_binary(len: usize) {
        BINARY_CLONES.fetch_add(1, Ordering::Relaxed);
        BINARY_TOTAL_BYTES.fetch_add(len as u64, Ordering::Relaxed);
    }
    pub fn snapshot() -> [(&'static str, u64); 12] {
        [
            ("list-clones", LIST_CLONES.load(Ordering::Relaxed)),
            ("list-total-items-cloned", LIST_TOTAL_LEN.load(Ordering::Relaxed)),
            ("list-clone-max-len", LIST_CLONE_MAX_LEN.load(Ordering::Relaxed)),
            ("record-clones", RECORD_CLONES.load(Ordering::Relaxed)),
            ("text-clones", TEXT_CLONES.load(Ordering::Relaxed)),
            ("binary-clones", BINARY_CLONES.load(Ordering::Relaxed)),
            ("binary-total-bytes-cloned", BINARY_TOTAL_BYTES.load(Ordering::Relaxed)),
            ("env-lookups (total)", ENV_LOOKUPS.load(Ordering::Relaxed)),
            ("env-list-lookups", ENV_LIST_LOOKUPS.load(Ordering::Relaxed)),
            ("env-list-lookup-total-len", ENV_LIST_LOOKUP_TOTAL_LEN.load(Ordering::Relaxed)),
            ("force-hits (forced thunks cloned)", FORCE_HITS.load(Ordering::Relaxed)),
            ("force-list-hits", FORCE_LIST_HITS.load(Ordering::Relaxed)),
        ]
    }
    pub fn bucket_snapshot() -> [u64; 16] {
        let mut out = [0u64; 16];
        for (i, b) in LIST_CLONE_BUCKETS.iter().enumerate() {
            out[i] = b.load(Ordering::Relaxed);
        }
        out
    }
}

#[derive(Debug)]
pub enum Value {
    Null,
    Logical(bool),
    Number(f64),
    /// Fixed-precision decimal. `mantissa` carries the unscaled integer
    /// value; `scale` is the count of decimal digits to the right of the
    /// point (so the represented number is `mantissa / 10^scale`).
    /// `precision` is the source column's declared precision — preserved
    /// so a parquet read→write round-trip emits the same Decimal(p, s)
    /// type. Single variant covers both Arrow Decimal128 (mantissa fits
    /// in the low 128 bits) and Decimal256.
    Decimal {
        mantissa: arrow::datatypes::i256,
        scale: i8,
        precision: u8,
    },
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

// Hand-written Clone so we can instrument the variants that carry owned
// allocations. With `profile-clones`, every clone bumps a counter; without
// it, this compiles to the same code `#[derive(Clone)]` would produce.
impl Clone for Value {
    fn clone(&self) -> Self {
        match self {
            Value::Null => Value::Null,
            Value::Logical(b) => Value::Logical(*b),
            Value::Number(n) => Value::Number(*n),
            Value::Decimal { mantissa, scale, precision } => Value::Decimal {
                mantissa: *mantissa, scale: *scale, precision: *precision,
            },
            Value::Text(s) => {
                #[cfg(feature = "profile-clones")]
                profile::bump_text();
                Value::Text(s.clone())
            }
            Value::Date(d) => Value::Date(*d),
            Value::Datetime(d) => Value::Datetime(*d),
            Value::Datetimezone(d) => Value::Datetimezone(*d),
            Value::Time(t) => Value::Time(*t),
            Value::Duration(d) => Value::Duration(*d),
            Value::Binary(b) => {
                #[cfg(feature = "profile-clones")]
                profile::bump_binary(b.len());
                Value::Binary(b.clone())
            }
            Value::List(xs) => {
                #[cfg(feature = "profile-clones")]
                profile::bump_list(xs.len());
                Value::List(xs.clone())
            }
            Value::Record(r) => {
                #[cfg(feature = "profile-clones")]
                profile::bump_record();
                Value::Record(r.clone())
            }
            Value::Table(t) => Value::Table(t.clone()),
            Value::Function(c) => Value::Function(c.clone()),
            Value::Type(t) => Value::Type(t.clone()),
            Value::WithMetadata { inner, meta } => Value::WithMetadata {
                inner: inner.clone(),
                meta: meta.clone(),
            },
            Value::Thunk(t) => Value::Thunk(t.clone()),
        }
    }
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
        params: Vec<(String /* name */, TypeRep, bool /* optional */)>,
        return_type: Box<TypeRep>,
    },
    /// Specific PQ numeric type names (Int64.Type, Int32.Type, Int16.Type,
    /// Int8.Type, Single.Type, Double.Type, Decimal.Type, Currency.Type,
    /// Percentage.Type). Stored as the literal type-name so Table.Schema can
    /// report the correct TypeName.
    NamedNumeric(&'static str),
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
    /// Deferred ODBC query plan. Constructed by `Odbc.DataSource`
    /// returning a navigation table where each row's `Data` cell is a
    /// `LazyOdbc` — the plan is fleshed out by foldable Table.* ops
    /// (SelectColumns, SelectRows, FirstN, ReorderColumns) without
    /// touching the wire. When forced, the SQL emitter renders the
    /// accumulated plan into a single SELECT statement which the
    /// connector's `force_fn` runs against the driver. Non-foldable
    /// downstream ops trigger a force boundary and proceed eagerly.
    LazyOdbc(LazyOdbcState),
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
    /// Per-output-column name override, parallel to `projection`. `None`
    /// at the outer level means "no renames anywhere — use schema field
    /// names as-is" (the common case, zero overhead). When `Some`, each
    /// inner `None` means "use schema field name", each `Some(s)` means
    /// "rename the column to `s` at force-time". Lets
    /// `Table.RenameColumns` / `PrefixColumns` / `DuplicateColumn`
    /// stay lazy by mutating only this list.
    pub output_names: Option<Vec<Option<String>>>,
    /// Total row count summed across row groups, cached from the
    /// footer at construction. Lets `Table.RowCount` return without
    /// decoding any data. Stays raw (pre-filter) even when
    /// `row_filter` is non-empty — RowCount on a filtered handle has
    /// to force the decode to be exact.
    pub num_rows: usize,
    /// Predicate filters to apply at decode time. Empty by default.
    /// `Table.SelectRows` on a LazyParquet input tries to translate
    /// foldable predicates into entries here; non-foldable predicates
    /// force the handle and filter eagerly in M-land. Indices in
    /// `RowFilter.source_col_idx` are into `schema.fields()` (i.e.
    /// the underlying parquet schema, NOT the projection), so they
    /// stay stable across column-narrowing operations.
    pub row_filter: Vec<RowFilter>,
}

/// A single predicate that can be evaluated against Parquet column
/// statistics (for row-group elimination) and against decoded Arrow
/// arrays (for per-row filtering). AND-combined with siblings in
/// `LazyParquetState.row_filter`. OR is currently expressed as a
/// non-foldable predicate that falls back to in-memory filtering.
#[derive(Debug, Clone)]
pub struct RowFilter {
    /// Index into the LazyParquet's underlying schema fields. Stays
    /// valid across SelectColumns / RemoveColumns even when the
    /// projection moves around it.
    pub source_col_idx: usize,
    pub op: FilterOp,
    pub scalar: FilterScalar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// `scalar` is ignored.
    IsNull,
    /// `scalar` is ignored.
    IsNotNull,
}

/// The constant side of a foldable predicate. Restricted to the
/// types we can compare against Parquet statistics without ambiguity.
#[derive(Debug, Clone)]
pub enum FilterScalar {
    Number(f64),
    Text(String),
    Logical(bool),
    Date(chrono::NaiveDate),
    Datetime(chrono::NaiveDateTime),
}

impl Value {
    /// Coerce a `Value::Decimal` (or `Number`) to f64, lossily. Used at
    /// the Number↔Decimal boundary where preserving precision isn't
    /// possible (Decimal × Number arithmetic, comparison vs Number,
    /// `Number.From` on a Decimal). Returns `None` for other variants.
    pub fn as_f64_lossy(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Decimal { mantissa, scale, .. } => Some(decimal_to_f64(*mantissa, *scale)),
            _ => None,
        }
    }
}

/// Convert a Decimal (mantissa, scale) to f64 — lossy for large
/// mantissas. The intermediate i128 path covers Decimal128 exactly and
/// truncates Decimal256 to its low 128 bits (with a `to_i128()` fallback
/// for values that fit). For values too large to fit i128 we fall
/// through to `i256::to_f64` if/when arrow exposes one; until then,
/// large Decimal256 → f64 returns inf for overflow.
pub(crate) fn decimal_to_f64(mantissa: arrow::datatypes::i256, scale: i8) -> f64 {
    let m = mantissa.to_i128().map(|x| x as f64).unwrap_or_else(|| {
        // Fallback for Decimal256 values that don't fit in i128:
        // build f64 from the high/low halves. Lossy at this magnitude
        // anyway (f64 has 53 bits of mantissa).
        let (low, high) = mantissa.to_parts();
        (high as f64) * (u128::MAX as f64 + 1.0) + (low as f64)
    });
    if scale == 0 {
        m
    } else if scale > 0 {
        m / 10f64.powi(scale as i32)
    } else {
        m * 10f64.powi(-(scale as i32))
    }
}

impl LazyParquetState {
    /// Effective output name for the column at position `i` in
    /// `projection` — applies any `output_names[i]` override or falls
    /// back to the source schema field name.
    pub fn effective_name(&self, i: usize) -> String {
        if let Some(ov) = self.output_names.as_ref().and_then(|v| v.get(i)) {
            if let Some(s) = ov {
                return s.clone();
            }
        }
        self.schema.field(self.projection[i]).name().clone()
    }
}

/// State for a `TableRepr::LazyOdbc` — a deferred ODBC query plan.
/// Each row of `Odbc.DataSource`'s navigation table holds one of
/// these as its `Data` cell. Foldable Table.* operations narrow the
/// plan in place (clone-and-replace); non-foldable ops force the plan
/// into an Arrow result at the call site.
#[derive(Clone)]
pub struct LazyOdbcState {
    /// Connection string passed to the driver. Owned because the
    /// `force_fn` closure captures it.
    pub connection_string: String,
    /// Bare table name. Drivers vary on whether `"catalog"."table"`
    /// qualification is accepted (DBISAM rejects it); the SQL emitter
    /// renders just `"table_name"` for portability.
    pub table_name: String,
    /// Column schema as discovered by the connector at navigation-table
    /// construction time (typically via `SELECT * ... WHERE 1=0` or
    /// `SQLDescribeCol`). Indices in `projection` are into this.
    pub schema: arrow::datatypes::SchemaRef,
    /// Column indices into `schema.fields()`, in output order. Initially
    /// `(0..schema.fields().len()).collect()`; narrowed by
    /// `Table.SelectColumns` / `RemoveColumns` / `ReorderColumns`.
    pub projection: Vec<usize>,
    /// Per-output-column rename overrides. Same shape as
    /// `LazyParquetState.output_names` — `None` means no renames; an
    /// inner `None` means "use the schema name at this position".
    pub output_names: Option<Vec<Option<String>>>,
    /// AND-conjoined predicate filters. Reuses the same `RowFilter`
    /// shape we use for parquet pushdown so the foldable-predicate
    /// extractor in stdlib can target both backends with one helper.
    pub where_filters: Vec<RowFilter>,
    /// Set by `Table.FirstN(_, n)`. Translates to `LIMIT n` (or `TOP n`
    /// for dialects that don't speak LIMIT — see the SQL emitter).
    pub limit: Option<usize>,
    /// Driver-side execution. The CLI shell builds this closure with
    /// `odbc_query_impl` captured; the WASM shell and `NoIoHost`
    /// never construct LazyOdbc values so they never need one. The
    /// closure receives the (potentially narrowed) state and returns
    /// the rendered Arrow batch.
    pub force_fn: std::rc::Rc<dyn Fn(&LazyOdbcState) -> Result<arrow::record_batch::RecordBatch, MError>>,
}

impl std::fmt::Debug for LazyOdbcState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyOdbcState")
            .field("connection_string", &"<elided>") // may contain secrets
            .field("table_name", &self.table_name)
            .field("projection_len", &self.projection.len())
            .field("where_filters_len", &self.where_filters.len())
            .field("limit", &self.limit)
            .finish()
    }
}

impl LazyOdbcState {
    pub fn effective_name(&self, i: usize) -> String {
        if let Some(ov) = self.output_names.as_ref().and_then(|v| v.get(i)) {
            if let Some(s) = ov {
                return s.clone();
            }
        }
        self.schema.field(self.projection[i]).name().clone()
    }

    /// Render the deferred plan into a portable-ish SQL SELECT statement.
    /// Dialect choices:
    /// - `"col"` for column quoting (works for most drivers)
    /// - Bare table name (no catalog/schema qualification — DBISAM
    ///   rejects "catalog"."table" syntax)
    /// - 0/1 for booleans (universal across drivers)
    /// - Single-quoted text literals with '' escape
    /// Limitations: no LIMIT (dialect-specific), no date/datetime
    /// literals yet (would need #'YYYY-MM-DD'# / ISO strings per dialect).
    pub fn render_sql(&self) -> String {
        use std::fmt::Write;
        let mut sql = String::with_capacity(64);
        sql.push_str("SELECT ");
        if self.projection.is_empty() {
            // No columns selected — emit COUNT(*) so the driver still
            // returns a usable row count. Shouldn't happen via fast
            // paths; here as a safety valve.
            sql.push_str("COUNT(*)");
        } else {
            for (i, &src_idx) in self.projection.iter().enumerate() {
                if i > 0 {
                    sql.push_str(", ");
                }
                // Source-schema name; renames are applied client-side
                // after the result lands (see materialise_lazy_odbc).
                let _ = write!(sql, "\"{}\"", self.schema.field(src_idx).name());
            }
        }
        let _ = write!(sql, " FROM \"{}\"", self.table_name);
        if !self.where_filters.is_empty() {
            sql.push_str(" WHERE ");
            for (i, f) in self.where_filters.iter().enumerate() {
                if i > 0 {
                    sql.push_str(" AND ");
                }
                render_filter(&mut sql, f, self.schema.as_ref());
            }
        }
        sql
    }
}

fn render_filter(out: &mut String, f: &RowFilter, schema: &arrow::datatypes::Schema) {
    use std::fmt::Write;
    let col_name = schema.field(f.source_col_idx).name();
    match f.op {
        FilterOp::IsNull => {
            let _ = write!(out, "\"{col_name}\" IS NULL");
            return;
        }
        FilterOp::IsNotNull => {
            let _ = write!(out, "\"{col_name}\" IS NOT NULL");
            return;
        }
        _ => {}
    }
    let op = match f.op {
        FilterOp::Eq => "=",
        FilterOp::Ne => "<>",
        FilterOp::Lt => "<",
        FilterOp::Le => "<=",
        FilterOp::Gt => ">",
        FilterOp::Ge => ">=",
        FilterOp::IsNull | FilterOp::IsNotNull => unreachable!(),
    };
    let _ = write!(out, "\"{col_name}\" {op} ");
    match &f.scalar {
        FilterScalar::Number(n) => {
            if n.fract() == 0.0 && n.is_finite() {
                let _ = write!(out, "{}", *n as i64);
            } else {
                let _ = write!(out, "{n}");
            }
        }
        FilterScalar::Text(s) => {
            out.push('\'');
            out.push_str(&s.replace('\'', "''"));
            out.push('\'');
        }
        FilterScalar::Logical(b) => out.push(if *b { '1' } else { '0' }),
        FilterScalar::Date(d) => {
            // ANSI: DATE 'YYYY-MM-DD'. Drivers that lack DATE-literal
            // syntax (DBISAM uses '#YYYY-MM-DD#') will need a dialect
            // override later; v1 emits ANSI.
            let _ = write!(out, "DATE '{}'", d);
        }
        FilterScalar::Datetime(dt) => {
            let _ = write!(out, "TIMESTAMP '{}'", dt);
        }
    }
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
                output_names: None,
                num_rows,
                row_filter: Vec::new(),
            }),
        })
    }

    pub fn column_names(&self) -> Vec<String> {
        match &self.repr {
            TableRepr::Arrow(b) => b.schema().fields().iter().map(|f| f.name().clone()).collect(),
            TableRepr::Rows { columns, .. } => columns.clone(),
            TableRepr::LazyParquet(s) => {
                (0..s.projection.len()).map(|i| s.effective_name(i)).collect()
            }
            TableRepr::LazyOdbc(s) => {
                (0..s.projection.len()).map(|i| s.effective_name(i)).collect()
            }
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
            // LazyOdbc has no cheap row count — the navtable carries
            // schema-only info, not row counts. Callers that need a
            // precise count should go through `Table.RowCount` which
            // emits `SELECT COUNT(*)` rather than `SELECT *`. Returning
            // 0 here would lie; usize::MAX is the conservative answer
            // that signals "must force to know".
            TableRepr::LazyOdbc(_) => usize::MAX,
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
            TableRepr::LazyOdbc(s) => s.projection.len(),
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
            | TableRepr::LazyOdbc(_)
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
            TableRepr::LazyOdbc(s) => materialise_lazy_odbc(s),
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
            TableRepr::LazyOdbc(s) => Ok(Cow::Owned(Self::from_arrow(materialise_lazy_odbc(s)?))),
            TableRepr::JoinView(jv) => Ok(Cow::Owned(materialise_join_view(jv)?)),
            TableRepr::ExpandView(ev) => Ok(Cow::Owned(materialise_expand_view(ev)?)),
        }
    }
}

/// Run a `LazyOdbc`'s plan through its `force_fn` and apply per-column
/// renames. Returns an Arrow RecordBatch — the same shape any other
/// successful Table.* operation would yield.
fn materialise_lazy_odbc(
    state: &LazyOdbcState,
) -> Result<arrow::record_batch::RecordBatch, MError> {
    let batch = (state.force_fn)(state)?;
    if let Some(onames) = state.output_names.as_ref() {
        if onames.iter().any(Option::is_some) {
            let schema = batch.schema();
            let new_fields: Vec<arrow::datatypes::Field> = schema
                .fields()
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let name = onames
                        .get(i)
                        .and_then(|o| o.clone())
                        .unwrap_or_else(|| f.name().clone());
                    arrow::datatypes::Field::new(name, f.data_type().clone(), f.is_nullable())
                })
                .collect();
            let new_schema = Arc::new(arrow::datatypes::Schema::new(new_fields));
            return arrow::record_batch::RecordBatch::try_new(
                new_schema,
                batch.columns().to_vec(),
            )
            .map_err(|e| MError::Other(format!("LazyOdbc rename: {e}")));
        }
    }
    Ok(batch)
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
            // Preserve per-column renames: pick the matching slots
            // from the existing output_names by position.
            let new_output_names = state.output_names.as_ref().map(|onames| {
                cols.iter().map(|&i| onames[i].clone()).collect()
            });
            Ok(Table {
                repr: TableRepr::LazyParquet(LazyParquetState {
                    bytes: state.bytes.clone(),
                    schema: state.schema.clone(),
                    projection: new_projection,
                    output_names: new_output_names,
                    num_rows: state.num_rows,
                    row_filter: state.row_filter.clone(),
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
        TableRepr::JoinView(_) | TableRepr::ExpandView(_) | TableRepr::LazyOdbc(_) => {
            // Force then narrow. For LazyOdbc this is the wrong shape
            // anyway — narrowing as a fold should happen in the stdlib
            // Table.SelectColumns fast path before reaching here.
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

    // Prefer Arrow encoding when every column is uniformly typed —
    // expand results often *are*, and Rows-backed tables can't be written
    // to Parquet. Genuinely mixed columns fall back to Rows naturally.
    super::stdlib::table::values_to_table(&out_names, &out_rows)
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
        // FullOuter: PQ order is matched-left rows first, then
        // unmatched-right rows (null-left), then unmatched-left rows
        // (each with a single-null-row nested table).
        3 => {
            let null_right_row: Vec<Value> = vec![Value::Null; right_names.len()];
            // 1. Matched left rows.
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
            // 2. Unmatched right rows.
            for &right_idx in &jv.unmatched_right {
                let mut row = null_left_row.clone();
                let nested_table = Table::from_rows(
                    right_names.clone(),
                    vec![read_row(right_table, right_idx as usize)?],
                );
                row.push(Value::Table(nested_table));
                out_rows.push(row);
            }
            // 3. Unmatched left rows.
            for (left_idx, match_indices) in jv.matches.iter().enumerate() {
                if !match_indices.is_empty() {
                    continue;
                }
                let nested_table = Table::from_rows(
                    right_names.clone(),
                    vec![null_right_row.clone()],
                );
                let mut row = read_row(left_table, left_idx)?;
                row.push(Value::Table(nested_table));
                out_rows.push(row);
            }
        }
        // LeftAnti: emit only left rows with NO match. PQ puts a
        // single all-null row in the nested column (not an empty
        // Table).
        4 => {
            let null_right_row: Vec<Value> = vec![Value::Null; right_names.len()];
            for (left_idx, match_indices) in jv.matches.iter().enumerate() {
                if !match_indices.is_empty() {
                    continue;
                }
                let nested_table = Table::from_rows(
                    right_names.clone(),
                    vec![null_right_row.clone()],
                );
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
        DataType::Decimal128(precision, scale) => {
            let raw = array
                .as_any()
                .downcast_ref::<Decimal128Array>()
                .expect("Decimal128")
                .value(row);
            Ok(Value::Decimal {
                mantissa: arrow::datatypes::i256::from_i128(raw),
                scale: *scale,
                precision: *precision,
            })
        }
        DataType::Decimal256(precision, scale) => {
            let raw = array
                .as_any()
                .downcast_ref::<Decimal256Array>()
                .expect("Decimal256")
                .value(row);
            Ok(Value::Decimal {
                mantissa: raw,
                scale: *scale,
                precision: *precision,
            })
        }
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

    // When row_filter is non-empty we need to (a) decode the columns
    // referenced by any filter so we can evaluate them, even if the
    // user's projection has narrowed past them; (b) survey row-group
    // statistics to drop groups that can't possibly match; (c) apply
    // filters per row after decode; (d) trim back to the user's
    // projection. Empty filter → original fast path.
    let extended_projection: Vec<usize> = if state.row_filter.is_empty() {
        state.projection.clone()
    } else {
        let mut ext = state.projection.clone();
        for f in &state.row_filter {
            if !ext.contains(&f.source_col_idx) {
                ext.push(f.source_col_idx);
            }
        }
        ext
    };

    let mut builder =
        ParquetRecordBatchReaderBuilder::try_new(state.bytes.as_ref().clone())
            .map_err(|e| MError::Other(format!("LazyParquet decode: {e}")))?;

    // Row-group elimination via column statistics.
    if !state.row_filter.is_empty() {
        let surviving = surviving_row_groups(builder.metadata(), &state.row_filter);
        if let Some(groups) = surviving {
            builder = builder.with_row_groups(groups);
        }
    }

    let mask = ProjectionMask::roots(
        builder.parquet_schema(),
        extended_projection.iter().copied(),
    );
    // Capture the builder's resolved schema before consuming it —
    // needed for empty-result handling when every row group is
    // eliminated (`reader.collect()` returns an empty Vec).
    let builder = builder.with_projection(mask);
    let reader_schema = builder.schema().clone();
    let reader = builder
        .build()
        .map_err(|e| MError::Other(format!("LazyParquet decode: {e}")))?;
    let batches: Vec<arrow::record_batch::RecordBatch> = reader
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| MError::Other(format!("LazyParquet decode: {e}")))?;
    let combined = match batches.len() {
        0 => arrow::record_batch::RecordBatch::new_empty(reader_schema),
        1 => batches.into_iter().next().expect("len == 1"),
        _ => arrow::compute::concat_batches(&batches[0].schema(), &batches)
            .map_err(|e| MError::Other(format!("LazyParquet decode concat: {e}")))?,
    };

    // ProjectionMask::roots returns batch columns in SCHEMA ORDER, not
    // the order we listed. Build a lookup from source_col_idx → batch
    // column position via the sorted/deduplicated extended projection.
    let mut sorted_ext: Vec<usize> = extended_projection.clone();
    sorted_ext.sort_unstable();
    sorted_ext.dedup();
    let batch_pos_of = |src: usize| -> usize {
        sorted_ext
            .iter()
            .position(|s| *s == src)
            .expect("source col was in extended projection")
    };

    // Per-row filter application.
    let filtered_batch = if state.row_filter.is_empty() {
        combined
    } else {
        apply_row_filters_at(&combined, &state.row_filter, &batch_pos_of)?
    };

    // Permute / trim to the user's projection order, dropping columns
    // that were only decoded for filter evaluation.
    let needs_reorder = state.projection.len() != filtered_batch.num_columns()
        || state
            .projection
            .iter()
            .enumerate()
            .any(|(out_pos, src)| batch_pos_of(*src) != out_pos);
    let trimmed = if !needs_reorder {
        filtered_batch
    } else {
        let keep_positions: Vec<usize> = state
            .projection
            .iter()
            .map(|src| batch_pos_of(*src))
            .collect();
        let schema = filtered_batch.schema();
        let new_fields: Vec<arrow::datatypes::Field> = keep_positions
            .iter()
            .map(|&p| (*schema.field(p)).clone())
            .collect();
        let new_cols: Vec<arrow::array::ArrayRef> = keep_positions
            .iter()
            .map(|&p| filtered_batch.column(p).clone())
            .collect();
        arrow::record_batch::RecordBatch::try_new(
            Arc::new(arrow::datatypes::Schema::new(new_fields)),
            new_cols,
        )
        .map_err(|e| MError::Other(format!("LazyParquet trim: {e}")))?
    };

    // Apply per-column renames if any are set.
    if let Some(onames) = state.output_names.as_ref() {
        if onames.iter().any(Option::is_some) {
            let schema = trimmed.schema();
            let new_fields: Vec<arrow::datatypes::Field> = schema
                .fields()
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let name = onames
                        .get(i)
                        .and_then(|o| o.clone())
                        .unwrap_or_else(|| f.name().clone());
                    arrow::datatypes::Field::new(name, f.data_type().clone(), f.is_nullable())
                })
                .collect();
            let new_schema = Arc::new(arrow::datatypes::Schema::new(new_fields));
            return arrow::record_batch::RecordBatch::try_new(
                new_schema,
                trimmed.columns().to_vec(),
            )
            .map_err(|e| MError::Other(format!("LazyParquet decode rename: {e}")));
        }
    }
    Ok(trimmed)
}

/// Walk row-group metadata, returning the indices of groups whose
/// (min, max, null_count) statistics could still satisfy every
/// RowFilter. Returns None if statistics are unavailable for any
/// filter column (then we skip group-level elimination and rely on
/// per-row filtering). Returns Some(vec) — possibly empty — when we
/// can answer definitively.
fn surviving_row_groups(
    metadata: &parquet::file::metadata::ParquetMetaData,
    filters: &[RowFilter],
) -> Option<Vec<usize>> {
    use parquet::file::statistics::Statistics;

    let n_groups = metadata.num_row_groups();
    let mut out: Vec<usize> = Vec::with_capacity(n_groups);
    for g in 0..n_groups {
        let rg = metadata.row_group(g);
        let mut keep = true;
        for f in filters {
            let col_meta = rg.column(f.source_col_idx);
            let stats = match col_meta.statistics() {
                Some(s) => s,
                None => continue, // no stats — assume could match
            };
            if !stats_can_match(stats, &f.op, &f.scalar) {
                keep = false;
                break;
            }
            let _ = stats; // silence unused if Statistics: clippy::let_unit later
            let _: &Statistics = stats;
        }
        if keep {
            out.push(g);
        }
    }
    Some(out)
}

/// Best-effort min/max/null comparison: returns `true` when the row
/// group *might* contain a matching row. Returns `true` (conservative)
/// for any statistics shape we don't know how to evaluate.
fn stats_can_match(
    stats: &parquet::file::statistics::Statistics,
    op: &FilterOp,
    scalar: &FilterScalar,
) -> bool {
    use parquet::file::statistics::Statistics as S;
    // Handle null-only filters first — they only need null_count
    // information, which works across all stat types.
    if matches!(op, FilterOp::IsNull) {
        return stats.null_count_opt().unwrap_or(1) > 0;
    }
    if matches!(op, FilterOp::IsNotNull) {
        // Group has a non-null row when row-count > null-count.
        let null_count = stats.null_count_opt().unwrap_or(0);
        let row_count = match stats {
            S::Boolean(_) | S::Int32(_) | S::Int64(_) | S::Int96(_)
            | S::Float(_) | S::Double(_) | S::ByteArray(_) | S::FixedLenByteArray(_) => {
                // Statistics doesn't expose row-count directly; assume not all null.
                u64::MAX
            }
        };
        return row_count > null_count;
    }
    // Min/max comparisons for the common numeric/text cases.
    match (stats, scalar) {
        (S::Int32(s), FilterScalar::Number(n)) => {
            let v = *n;
            let lo = s.min_opt().copied().map(|x| x as f64);
            let hi = s.max_opt().copied().map(|x| x as f64);
            range_can_match(lo, hi, *op, v)
        }
        (S::Int64(s), FilterScalar::Number(n)) => {
            let v = *n;
            let lo = s.min_opt().copied().map(|x| x as f64);
            let hi = s.max_opt().copied().map(|x| x as f64);
            range_can_match(lo, hi, *op, v)
        }
        (S::Float(s), FilterScalar::Number(n)) => {
            let v = *n;
            let lo = s.min_opt().copied().map(|x| x as f64);
            let hi = s.max_opt().copied().map(|x| x as f64);
            range_can_match(lo, hi, *op, v)
        }
        (S::Double(s), FilterScalar::Number(n)) => {
            let v = *n;
            let lo = s.min_opt().copied();
            let hi = s.max_opt().copied();
            range_can_match(lo, hi, *op, v)
        }
        // ByteArray comparisons for text — only handle Eq/Ne/Lt/Gt as
        // lexicographic. The min/max bytes are UTF-8 in practice.
        (S::ByteArray(s), FilterScalar::Text(t)) => {
            let v = t.as_bytes();
            let lo = s.min_opt().map(|b| b.data());
            let hi = s.max_opt().map(|b| b.data());
            text_range_can_match(lo, hi, *op, v)
        }
        // Unknown stat × scalar combos: conservatively say yes.
        _ => true,
    }
}

fn range_can_match(lo: Option<f64>, hi: Option<f64>, op: FilterOp, v: f64) -> bool {
    match op {
        FilterOp::Eq => {
            // [v in [lo..=hi]]
            !(lo.is_some_and(|l| v < l) || hi.is_some_and(|h| v > h))
        }
        FilterOp::Ne => {
            // [lo..=hi] != {v} iff range has more than one value or != v
            !(lo == Some(v) && hi == Some(v))
        }
        FilterOp::Lt => lo.is_none_or(|l| l < v),
        FilterOp::Le => lo.is_none_or(|l| l <= v),
        FilterOp::Gt => hi.is_none_or(|h| h > v),
        FilterOp::Ge => hi.is_none_or(|h| h >= v),
        FilterOp::IsNull | FilterOp::IsNotNull => true,
    }
}

fn text_range_can_match(lo: Option<&[u8]>, hi: Option<&[u8]>, op: FilterOp, v: &[u8]) -> bool {
    match op {
        FilterOp::Eq => !(lo.is_some_and(|l| v < l) || hi.is_some_and(|h| v > h)),
        FilterOp::Ne => !(lo == Some(v) && hi == Some(v)),
        FilterOp::Lt => lo.is_none_or(|l| l < v),
        FilterOp::Le => lo.is_none_or(|l| l <= v),
        FilterOp::Gt => hi.is_none_or(|h| h > v),
        FilterOp::Ge => hi.is_none_or(|h| h >= v),
        FilterOp::IsNull | FilterOp::IsNotNull => true,
    }
}

/// Per-row filter application after decode. Returns a new RecordBatch
/// with only the rows matching every filter (AND semantics).
/// `batch_pos_of` maps a source_col_idx to its position in the batch
/// (the parquet reader returns columns in schema order, not in the
/// order we listed them in the projection mask).
fn apply_row_filters_at(
    batch: &arrow::record_batch::RecordBatch,
    filters: &[RowFilter],
    batch_pos_of: &dyn Fn(usize) -> usize,
) -> Result<arrow::record_batch::RecordBatch, MError> {
    use arrow::array::BooleanArray;
    use arrow::compute::filter_record_batch;

    let n_rows = batch.num_rows();
    let mut mask: Vec<bool> = vec![true; n_rows];
    for f in filters {
        let pos = batch_pos_of(f.source_col_idx);
        let column = batch.column(pos);
        for row in 0..n_rows {
            if !mask[row] {
                continue;
            }
            mask[row] = cell_matches_filter(column.as_ref(), row, &f.op, &f.scalar);
        }
    }
    let bool_mask = BooleanArray::from(mask);
    filter_record_batch(batch, &bool_mask)
        .map_err(|e| MError::Other(format!("LazyParquet filter: {e}")))
}

fn cell_matches_filter(
    column: &dyn arrow::array::Array,
    row: usize,
    op: &FilterOp,
    scalar: &FilterScalar,
) -> bool {
    use arrow::array::*;
    use arrow::datatypes::DataType;

    let is_null = column.is_null(row);
    match op {
        FilterOp::IsNull => return is_null,
        FilterOp::IsNotNull => return !is_null,
        _ => {}
    }
    if is_null {
        // Comparisons against null: M's 3-valued logic — null
        // comparison result is null, treated as "not matched".
        return false;
    }
    match (column.data_type(), scalar) {
        (DataType::Boolean, FilterScalar::Logical(b)) => {
            let v = column
                .as_any()
                .downcast_ref::<BooleanArray>()
                .unwrap()
                .value(row);
            cmp_partial(v.cmp(b), *op)
        }
        (DataType::Int8, FilterScalar::Number(n)) => {
            let v = column.as_any().downcast_ref::<Int8Array>().unwrap().value(row) as f64;
            cmp_f64(v, *n, *op)
        }
        (DataType::Int16, FilterScalar::Number(n)) => {
            let v = column.as_any().downcast_ref::<Int16Array>().unwrap().value(row) as f64;
            cmp_f64(v, *n, *op)
        }
        (DataType::Int32, FilterScalar::Number(n)) => {
            let v = column.as_any().downcast_ref::<Int32Array>().unwrap().value(row) as f64;
            cmp_f64(v, *n, *op)
        }
        (DataType::Int64, FilterScalar::Number(n)) => {
            let v = column.as_any().downcast_ref::<Int64Array>().unwrap().value(row) as f64;
            cmp_f64(v, *n, *op)
        }
        (DataType::UInt8, FilterScalar::Number(n)) => {
            let v = column.as_any().downcast_ref::<UInt8Array>().unwrap().value(row) as f64;
            cmp_f64(v, *n, *op)
        }
        (DataType::UInt16, FilterScalar::Number(n)) => {
            let v = column.as_any().downcast_ref::<UInt16Array>().unwrap().value(row) as f64;
            cmp_f64(v, *n, *op)
        }
        (DataType::UInt32, FilterScalar::Number(n)) => {
            let v = column.as_any().downcast_ref::<UInt32Array>().unwrap().value(row) as f64;
            cmp_f64(v, *n, *op)
        }
        (DataType::UInt64, FilterScalar::Number(n)) => {
            let v = column.as_any().downcast_ref::<UInt64Array>().unwrap().value(row) as f64;
            cmp_f64(v, *n, *op)
        }
        (DataType::Float32, FilterScalar::Number(n)) => {
            let v = column.as_any().downcast_ref::<Float32Array>().unwrap().value(row) as f64;
            cmp_f64(v, *n, *op)
        }
        (DataType::Float64, FilterScalar::Number(n)) => {
            let v = column.as_any().downcast_ref::<Float64Array>().unwrap().value(row);
            cmp_f64(v, *n, *op)
        }
        (DataType::Utf8, FilterScalar::Text(t)) => {
            let v = column.as_any().downcast_ref::<StringArray>().unwrap().value(row);
            cmp_partial(v.cmp(t.as_str()), *op)
        }
        // Type mismatch (Text scalar vs Int column etc) — never matches.
        _ => false,
    }
}

fn cmp_f64(v: f64, scalar: f64, op: FilterOp) -> bool {
    match op {
        FilterOp::Eq => v == scalar,
        FilterOp::Ne => v != scalar,
        FilterOp::Lt => v < scalar,
        FilterOp::Le => v <= scalar,
        FilterOp::Gt => v > scalar,
        FilterOp::Ge => v >= scalar,
        FilterOp::IsNull | FilterOp::IsNotNull => unreachable!(),
    }
}

fn cmp_partial(ord: std::cmp::Ordering, op: FilterOp) -> bool {
    use std::cmp::Ordering;
    match (ord, op) {
        (Ordering::Equal, FilterOp::Eq) => true,
        (Ordering::Equal, FilterOp::Le | FilterOp::Ge) => true,
        (Ordering::Less, FilterOp::Lt | FilterOp::Le | FilterOp::Ne) => true,
        (Ordering::Greater, FilterOp::Gt | FilterOp::Ge | FilterOp::Ne) => true,
        _ => false,
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

#[cfg(test)]
mod odbc_sql_tests {
    use super::*;

    fn dummy_state(filters: Vec<RowFilter>) -> LazyOdbcState {
        use arrow::datatypes::{DataType, Field, Schema};
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("price", DataType::Float64, true),
        ]));
        LazyOdbcState {
            connection_string: "DSN=test".into(),
            table_name: "customers".into(),
            schema,
            projection: vec![0, 1, 2],
            output_names: None,
            where_filters: filters,
            limit: None,
            force_fn: std::rc::Rc::new(|_| {
                panic!("force_fn must not be called in render-only tests")
            }),
        }
    }

    #[test]
    fn render_select_star_equivalent() {
        let sql = dummy_state(vec![]).render_sql();
        assert_eq!(sql, r#"SELECT "id", "name", "price" FROM "customers""#);
    }

    #[test]
    fn render_with_int_filter() {
        let f = RowFilter {
            source_col_idx: 0,
            op: FilterOp::Gt,
            scalar: FilterScalar::Number(100.0),
        };
        let sql = dummy_state(vec![f]).render_sql();
        assert_eq!(
            sql,
            r#"SELECT "id", "name", "price" FROM "customers" WHERE "id" > 100"#
        );
    }

    #[test]
    fn render_with_text_filter_escapes_quotes() {
        let f = RowFilter {
            source_col_idx: 1,
            op: FilterOp::Eq,
            scalar: FilterScalar::Text("O'Brien".into()),
        };
        let sql = dummy_state(vec![f]).render_sql();
        assert_eq!(
            sql,
            r#"SELECT "id", "name", "price" FROM "customers" WHERE "name" = 'O''Brien'"#
        );
    }

    #[test]
    fn render_with_null_filter() {
        let f = RowFilter {
            source_col_idx: 2,
            op: FilterOp::IsNull,
            scalar: FilterScalar::Logical(false),
        };
        let sql = dummy_state(vec![f]).render_sql();
        assert_eq!(
            sql,
            r#"SELECT "id", "name", "price" FROM "customers" WHERE "price" IS NULL"#
        );
    }

    #[test]
    fn render_with_two_filters_anded() {
        let filters = vec![
            RowFilter {
                source_col_idx: 0,
                op: FilterOp::Ge,
                scalar: FilterScalar::Number(10.0),
            },
            RowFilter {
                source_col_idx: 2,
                op: FilterOp::Le,
                scalar: FilterScalar::Number(99.95),
            },
        ];
        let sql = dummy_state(filters).render_sql();
        assert_eq!(
            sql,
            r#"SELECT "id", "name", "price" FROM "customers" WHERE "id" >= 10 AND "price" <= 99.95"#
        );
    }

    #[test]
    fn render_with_narrowed_projection() {
        let mut state = dummy_state(vec![]);
        state.projection = vec![1]; // just "name"
        let sql = state.render_sql();
        assert_eq!(sql, r#"SELECT "name" FROM "customers""#);
    }
}
