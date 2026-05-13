# 09 — Lazy Tables (Stage A design note)

**Status:** Design proposal — nothing implemented. Shred or approve.
**Origin:** WASM demo session 2026-05-12. Empirical confirmation in
`project_lazy_tables_and_quack` memory; "user can't be expected to
hand-project every right-side table" in `feedback_no_semantics_changing_optimisation`.

## 1. The problem in one paragraph

Power Query's GUI emits M of the shape "load every column, do work, drop most
columns at the end." `parqs/mini-wasm.m` (the WASM-demo corpus M) does exactly
this: `Parquet.Document` reads every column of a 148K-row analysis table,
joins it twice via `Table.NestedJoin` against a 134-col customer and 163-col
product table, and only at the very end does `Table.RemoveColumns` /
`Table.ExpandTableColumn` reveal that just two of those columns
(`cpyname`, `desc1`) actually matter. The current evaluator faithfully
materialises every cell of every column before the trim, costing ~32s
native and OOM in WASM. The user is unwilling to insert manual
`Table.SelectColumns` calls in every query they want to run — that pushes
implementation pain onto the M author and breaks the "Excel-generated M
runs unmodified" promise.

## 2. Scope decision (the question the user has to answer)

There are two coherent stopping points. Pick one before any code lands.

### 2a. Stage A "narrow" — lazy `Parquet.Document` only

`Parquet.Document(path)` returns a `LazyParquet` table handle that has read
the parquet footer (so schema is known and `column_names` / `num_rows` work)
but holds the row data only as the original byte buffer. Operations that
only touch schema (`Table.ColumnNames`, `Table.Schema`, `Table.RowCount`,
`Table.HasColumns`, `Table.IsEmpty`) never force. Column-projecting ops
(`Table.SelectColumns`, `Table.RemoveColumns`, `Table.ReorderColumns`)
narrow an internal `ProjectionMask` and stay lazy. **Everything else
forces** the handle into a regular `Arrow` table with the accumulated mask
applied, reading only the selected columns from the parquet bytes.

**Benefit:** queries with explicit `Table.SelectColumns(Parquet.Document(p), {cols})`
or `Table.RemoveColumns(Parquet.Document(p), {cols})` near the top get
column pushdown for free.

**Doesn't help:** `parqs/mini-wasm.m` directly. The corpus pattern joins
*before* projecting. By the time `RemoveColumns` runs, the handle has
already been forced by `Table.NestedJoin` and the unused columns are
already loaded.

### 2b. Stage A "wide" — laziness propagates through `Table.NestedJoin`

Same as 2a, plus a new `TableRepr::JoinView` variant. `Table.NestedJoin`
on a `LazyParquet` (or `Arrow`) right side records the matched-row
indices per left row but doesn't materialise the right rows. The nested
column in the join result is a `JoinView(handle, indices_per_outer_row)`.
When `Table.ExpandTableColumn(joined, "custs", {"cpyname"}, …)` later
reads `cpyname`, *only that column* gets pulled from the right side's
underlying handle. The columns the user never expands are never read.

**Benefit:** addresses the corpus pattern directly. The mini-wasm.m demo
should drop from 32s native (and OOM WASM) to seconds and fit in WASM
memory comfortably.

**Cost:** bigger architectural surface. Three table reprs become four,
and `JoinView` has to participate in the same projection-aware /
force-on-entry classification as `LazyParquet`.

**Recommendation:** 2b. 2a is interesting but doesn't actually move the
needle on the workloads we just saw fail. Half a fix is worse than no
fix because it spends architectural budget without changing observable
outcomes.

## 3. RT (referential transparency) argument

Per `feedback_no_semantics_changing_optimisation`: any optimisation
must preserve observable semantics. The proposed laziness does:

- `LazyParquet` produces the same `Value` for any cell as the
  current eager `Parquet.Document` would. The mask only controls
  *when* columns are decoded, not *what* values they decode to.
- `JoinView` produces the same `Value` for any cell as the current
  `Table.NestedJoin` would. The match-indices are the same set the
  hash join already computes; the nested-table shape (column names,
  row order, cell values) is identical when fully forced.
- Forcing happens implicitly on any op that's not in the
  projection-aware allowlist. So an M author who never thinks about
  laziness sees identical results to the eager evaluator.

What we are **not** doing:
- Not modifying return schemas based on downstream uses (RT violation,
  rejected 2026-05-12).
- Not silently dropping rows the M source didn't tell us to drop.
- Not introducing user-visible API for laziness (`Table.Buffer` already
  forces; we lean on that as the explicit "materialise now" signal if
  ever needed).

## 4. Data model

```rust
pub enum TableRepr {
    Arrow(RecordBatch),
    Rows { columns: Vec<String>, rows: Vec<Vec<Value>> },
    LazyParquet(LazyParquetState),     // new
    JoinView(JoinViewState),            // new — only for Stage 2b
}

pub struct LazyParquetState {
    /// Original bytes the parquet crate reads from. Shared via Arc so
    /// cloning a `Table` doesn't duplicate (parquet files can be 200MB).
    bytes: Arc<bytes::Bytes>,
    /// Full schema read from the footer at construction time. Cheap.
    schema: SchemaRef,
    /// Indices into `schema` that downstream code might still consume.
    /// Mutated (via clone-and-replace) by Table.SelectColumns etc.
    /// `None` means "all columns" (the initial state).
    projection: Option<Vec<usize>>,
    /// Optional row-group filter for Stage A.5 (predicate pushdown).
    /// Out of scope for Stage A; field shape future-proofed.
    row_groups: Option<Vec<usize>>,
}

pub struct JoinViewState {
    /// The outer-left table after the join (already eager).
    /// Includes the placeholder nested column at `nested_col_idx`.
    left: Box<TableRepr>,
    nested_col_idx: usize,
    /// The right side, held lazy if it came in lazy.
    right: Arc<Table>,
    right_key_idx: usize,
    /// For each outer-row, the right-row indices it joins to.
    /// Built by the hash-join pass; what was previously cloned eagerly.
    matches: Vec<Vec<u32>>,
    new_column_name: String,
}
```

`Arc` on `bytes` and `Table` keeps cloning cheap. The mask is a `Vec<usize>`
not a bit-mask — column counts are bounded (~200 in practice), order
matters (it's the resulting schema), and `Vec` is what
`parquet::arrow::ProjectionMask::roots` expects.

## 5. Forcing semantics

Add a private helper:

```rust
fn force(table: &Table) -> Result<Cow<'_, Table>, MError>
```

For `Arrow` / `Rows` it's a no-op borrow. For `LazyParquet` it constructs
`ParquetRecordBatchReaderBuilder` with `with_projection(...)` set from
the mask and produces an `Arrow`-backed `Table`. For `JoinView`,
materialises the right side, walks `matches` to build the nested
tables, and substitutes them into the outer rows.

The current `Table::as_arrow()` and `try_to_arrow()` would gain an
internal force step before downcasting.

The `IoHost::parquet_read` signature stays unchanged — it still returns
a `Value::Table`. The lazy state is constructed inside that method by
the WASM and CLI shells. The trait does not need to know about lazy:
this is an evaluator-internal optimisation.

## 6. Classification of stdlib Table-consuming functions

The audit (§10) classifies every binding into four buckets:

- **(P)** **Projection-aware** — can transform a `LazyParquet`/`JoinView`
  into another `LazyParquet`/`JoinView` without forcing. Stage A's win
  hinges on these.
- **(S)** **Schema-only** — only needs column names/types/count, no row
  data. Trivially lazy-safe.
- **(R)** **Row-bound** — needs row data. Forces on entry, becomes
  Arrow/Rows.
- **(C)** **Constructor** — produces a table from non-table inputs.
  Not a consumer. Not affected.
- **(I)** **Identity passthrough** — Power Query metadata stubs that
  currently return the input unchanged. Stay unchanged; if input is
  lazy, output is lazy.

The "P" set is small but high-leverage. The "S" set is medium and cheap
to flag. Everything else falls into "R" and gets a one-line
`let table = force(&table)?;` at the top of the function body.

## 7. Implementation order (if 2b is approved)

1. Add `TableRepr::LazyParquet` and `force()`. Wire `IoHost::parquet_read`
   in both shells to return it. Add force-on-entry to every "R" function.
   At this point all queries still work, behaviour is unchanged from
   today's perspective, perf may be marginally worse due to the force
   indirection (negligible).
2. Wire the **P** set (SelectColumns, RemoveColumns, ReorderColumns)
   to narrow the mask instead of forcing. Add tests asserting that
   `Table.SelectColumns(Parquet.Document(p), {col})` only reads `col`
   bytes from the parquet (use a memory-counting `IoHost` wrapper).
3. Wire the **S** set (ColumnNames, Schema, RowCount, ColumnCount,
   HasColumns, IsEmpty). These never need to force.
4. Add `TableRepr::JoinView`. Modify `nested_join` to return a JoinView
   when the right side is `LazyParquet` (or `Arrow` — same logic).
   Modify `expand_table_column` and `select_columns` to recognise
   JoinView nested cells and pull only the needed columns from the
   right handle.
5. Benchmark against `parqs/mini-cli.m` and `parqs/mini-wasm.m`. Goal:
   sub-second native, sub-10s WASM, fits in 1GB linear memory.

## 8. Open questions

- **`Table.Buffer` semantics.** PQ uses `Table.Buffer` as an explicit
  "force this now and cache it" signal. Should our impl be the explicit
  way to opt out of laziness? Currently it's an identity passthrough.
- **`JoinView` and `ExpandTableColumn` with multiple expanded columns.**
  If the M source expands two columns at once via
  `ExpandTableColumn(t, "custs", {"a","b"}, {"a","b"})`, we read both
  from the lazy right side. Fine. But what about *sequential* expands
  off the same nested column? Probably need to materialise lazily-then-
  cached once the first expand happens.
- **Sort/Group on a `LazyParquet` key column.** Forces, but could
  theoretically narrow the projection to just the key column first if
  the rest of the table is discarded. Out of scope for Stage A; flag for
  Stage A.5.
- **Predicate pushdown on `Table.SelectRows`.** Parquet has row-group
  statistics that let you skip whole row groups based on min/max for
  simple predicates. Real value-add but real complexity — needs
  predicate-to-stats translation. Defer.
- **Memory ownership for `bytes::Bytes`.** Currently the WASM `IoHost`
  copies parquet bytes from a `Uint8Array` into a `Vec<u8>` into a
  `Bytes`. With laziness, that buffer lives longer (until forced). Worth
  re-checking the copy chain — probably fine but worth one round of
  scrutiny.
- **Mask shape.** Proposed mask is `Vec<usize>` (source column index in
  output order). This handles Select/Remove/Reorder cleanly. Three more
  bindings (`RenameColumns`, `PrefixColumns`, `DuplicateColumn`) would
  also be projection-aware if the mask were richer:
  `Vec<MaskEntry { source_idx: usize, output_name: String }>` and
  `DuplicateColumn` would emit two entries sharing a `source_idx`.
  This is a small scope expansion (~30 LOC) but it widens the "P"
  set from 3 to 6 bindings and lets renames stay lazy. Worth doing if
  the corpus uses renames/prefixes a lot — currently `parqs/mini-wasm.m`
  doesn't, so the simple mask suffices.

## 9. Estimated diff size

(Order of magnitude only, no time estimate per `feedback_no_time_estimates`.)

- Stage 2a alone: ~400 LOC. New variant + force + P/S/R classification of
  ~53 stdlib bindings (most are one-line additions). Plus tests.
- Stage 2b: ~600-900 LOC on top of 2a. `JoinView` is mechanically simple
  but touches NestedJoin's existing hash-join code carefully.

## 10. Audit — stdlib bindings classified

Notation: **(P)** projection-aware, **(S)** schema-only, **(R)** row-bound,
**(C)** constructor, **(I)** identity stub. The class is what the function
*should be* under Stage 2b, not what it is today. Today every "P" and "S"
function operates on already-materialised tables.

### High-leverage projection-aware

- **(P)** `Table.SelectColumns` — narrows mask
- **(P)** `Table.RemoveColumns` — narrows mask (complement of SelectColumns)
- **(P)** `Table.ReorderColumns` — reorders mask
- **(R)** `Table.RenameColumns` — would be P if the mask carried output
  names per entry; with `Vec<usize>` mask it forces (see §8 — mask shape).
- **(R)** `Table.PrefixColumns` — same caveat as RenameColumns
- **(R)** `Table.DuplicateColumn` — would be P if the mask allowed
  multiple entries to reference the same source index with different
  output names; with `Vec<usize>` mask it forces (see §8 — mask shape).

### Schema-only

- **(S)** `Table.ColumnNames`
- **(S)** `Table.ColumnCount`
- **(S)** `Table.Schema`
- **(S)** `Table.HasColumns`
- **(S)** `Table.ColumnsOfType` — needs column data types only, from schema
- **(S)** `Table.IsEmpty` — uses `num_rows() == 0`, schema-only
- **(S)** `Table.RowCount` — parquet footer carries row count, no decode
- **(S)** `Table.ApproximateRowCount` — same

### Row-bound (force on entry)

Most of the catalogue:
- **(R)** `Table.SelectRows`, `Table.Sort`, `Table.Distinct`, `Table.Group`
- **(R)** `Table.AddColumn`, `Table.TransformColumns`, `Table.TransformColumnTypes`, `Table.TransformRows`, `Table.TransformColumnNames`
- **(R)** `Table.AddIndexColumn`, `Table.AddRankColumn`, `Table.AddJoinColumn`, `Table.AddFuzzyClusterColumn`
- **(R)** `Table.Join`, `Table.FuzzyJoin` — flat join, materialises both sides
- **(R)** `Table.NestedJoin` — under 2a, force right side. Under 2b, **special**: produces a `JoinView` (see §4).
- **(R)** `Table.ExpandRecordColumn`, `Table.ExpandListColumn`
- **(R)** `Table.ExpandTableColumn` — under 2b, **special**: if input column is a `JoinView` nested cell, pull only requested columns from the right handle.
- **(R)** `Table.Unpivot`, `Table.UnpivotOtherColumns`, `Table.Pivot`, `Table.Transpose`
- **(R)** `Table.FirstN`, `Table.Skip`, `Table.Range`, `Table.SplitAt`, `Table.AlternateRows`, `Table.First`, `Table.Last`, `Table.FirstValue`, `Table.SingleRow`, `Table.ReverseRows`
- **(R)** `Table.RemoveFirstN`, `Table.RemoveLastN`, `Table.RemoveRows`, `Table.RemoveMatchingRows`, `Table.RemoveRowsWithErrors`, `Table.ReplaceMatchingRows`, `Table.ReplaceRows`, `Table.ReplaceValue`, `Table.ReplaceErrorValues`
- **(R)** `Table.Min`, `Table.Max`, `Table.MinN`, `Table.MaxN`, `Table.AggregateTableColumn`
- **(R)** `Table.Contains`, `Table.ContainsAll`, `Table.ContainsAny`, `Table.IsDistinct`, `Table.PositionOf`, `Table.PositionOfAny`, `Table.MatchesAllRows`, `Table.MatchesAnyRows`, `Table.FindText`
- **(R)** `Table.Keys`, `Table.FillUp`, `Table.FillDown`, `Table.Repeat`, `Table.Profile`
- **(R)** `Table.CombineColumns`, `Table.CombineColumnsToRecord`, `Table.SplitColumn`
- **(R)** `Table.Combine` — forces, but could in principle stay lazy by maintaining a list of handles. Out of scope.
- **(R)** `Table.ToRecords`, `Table.ToColumns`, `Table.ToList`, `Table.ToRows`, `Table.Column`
- **(R)** `Table.Split`, `Table.Partition`, `Table.PartitionKey`, `Table.PartitionValues`, `Table.FilterWithDataTable`, `Table.SelectRowsWithErrors`, `Table.PromoteHeaders`, `Table.DemoteHeaders`, `Table.InsertRows`
- **(R)** `Table.Buffer` — currently identity, see §8.

### Constructors

- **(C)** `#table`
- **(C)** `Table.FromRows`, `Table.FromRecords`, `Table.FromColumns`, `Table.FromList`, `Table.FromValue`, `Table.FromPartitions`

### Identity passthroughs

PQ-specific stubs that compile but do nothing meaningful. Stay lazy if
input is lazy, no special handling needed.

- **(I)** `Table.AddKey`, `Table.ReplaceKeys`, `Table.ConformToPageReader`,
  `Table.StopFolding`, `Table.ReplaceRelationshipIdentity`,
  `Table.WithErrorContext`, `Table.ReplacePartitionKey`,
  `Table.View`, `Table.ViewError`, `Table.ViewFunction`
- **(I)** `Table.FuzzyGroup`, `Table.FuzzyNestedJoin`,
  `Table.AddFuzzyClusterColumn` — actually error with NotImplemented, not
  identity, but irrelevant to laziness either way.

### Totals

- 3 P (projection-aware with simple `Vec<usize>` mask, where Stage A's
  column pushdown lives) — 6 if the richer mask in §8 is adopted
- 8 S (schema-only, "free" once classified)
- ~70 R (force on entry — most of the catalogue)
- 7 C (constructors, untouched)
- ~10 I (stubs, untouched)

Stage 2b adds special handling for **2 functions** beyond the P/S
classification: `Table.NestedJoin` (produces JoinView) and
`Table.ExpandTableColumn` (consumes JoinView lazily). Everything else
follows the same force-on-entry pattern as today.
