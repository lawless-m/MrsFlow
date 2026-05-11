# 07 — Evaluator Design

The evaluator turns parsed AST into M values. Like the lexer and parser, it ships in two implementations: the Rust evaluator is the product, and a Prolog evaluator companion in `tools/grammar-fuzz/evaluator.pl` is the differential oracle for evaluation semantics. Same pattern that worked for the parser — independent encoding of the spec, validated against each other on every change.

## Architecture position

The evaluator lives entirely in `mrsflow-core`. It is **synchronous and pure** in the sense that it does no IO of its own — but stdlib functions like `Odbc.Query`, `Odbc.DataSource`, `Parquet.Document` need to reach external systems. Those reach out through an `IoHost` trait that the shell provides.

```
┌─ shell (CLI) ─────────────────────────┐
│  CliIoHost { real ODBC + filesystem } │
│                  ↓                    │
│  ┌─ mrsflow-core ───────────────┐    │
│  │  evaluator (sync, no IO)     │    │
│  │  stdlib fns call &dyn IoHost │    │
│  └──────────────────────────────┘    │
└───────────────────────────────────────┘

┌─ shell (WASM) ────────────────────────┐
│  WasmIoHost { fetch+parquet-wasm,     │
│               odbc methods all err }  │
│                  ↓                    │
│  ┌─ mrsflow-core (same crate) ──┐    │
│  └──────────────────────────────┘    │
└───────────────────────────────────────┘
```

This keeps `mrsflow-core` itself IO-free (still trivially WASM-compilable, no `tokio` leak, no filesystem assumptions) while letting stdlib functions reach external systems via a small, well-defined surface.

### The IoHost trait

```rust
pub trait IoHost {
    fn parquet_read(&self, path: &str) -> Result<arrow::RecordBatch, IoError>;
    fn parquet_write(&self, path: &str, batch: &arrow::RecordBatch) -> Result<(), IoError>;
    fn odbc_query(&self, conn: &str, sql: &str, opts: Option<&Value>) -> Result<arrow::RecordBatch, IoError>;
    fn odbc_data_source(&self, conn: &str, opts: Option<&Value>) -> Result<NavTable, IoError>;
}
```

`NavTable` is the navigation-table shape `Odbc.DataSource` returns: rows of `{Name, Schema, Kind, Data}` where `Data` is a closure-like thing the evaluator wraps in a `Value::Thunk` so the underlying `SELECT *` only fires on access.

Per-shell implementations:

| Method | CLI | WASM |
|---|---|---|
| `parquet_read` | `parquet` crate | `parquet-wasm` or DuckDB-Wasm bridge |
| `parquet_write` | `parquet` crate | `Err(NotSupported)` (browsers don't write files; result returns to JS) |
| `odbc_query` | `odbc-api` crate | `Err(NotSupported)` |
| `odbc_data_source` | `odbc-api` catalog calls | `Err(NotSupported)` |

The intrinsic environment built at evaluator startup binds `Odbc.Query`, `Odbc.DataSource`, etc. to stdlib functions only on shells where the corresponding `IoHost` methods are real. On WASM, those names are simply not in scope — accessing them produces a "name not in scope" error, same as misspelling.

## Load-bearing decisions

These shape every `Value` variant, every operator, every stdlib function, and every line of the Prolog companion. Locking them in before either implementation starts.

### 1. Laziness

The spec is genuinely lazy. `let a = error "x", b = 1 in b` evaluates to `1` — the unused binding never forces, the error never raises. Record fields are independently lazy too: in `[a = error "x", b = 1]`, accessing `[b]` succeeds without forcing `a`.

**Decision:** thunked values from day one.

- Rust: `Value` carries a `Value::Thunk(Rc<RefCell<ThunkState>>)` variant. `ThunkState` is either `Pending(Expr, Env)` or `Forced(Value)`. Forcing memoises; second access is direct.
- Prolog: thunks as `thunk(Expr, Env)` terms. `force(Thunked, Forced)` predicate evaluates if pending. Memoisation via `assert` of a forced fact keyed on a thunk ID, or via destructive `nb_setarg` on a mutable cell — TBD when we get there.

`let` bindings are thunked at binding time and forced on reference. Record field values are thunked at construction time. Function arguments are forced at call time (M is not call-by-name, just lazy in `let`/record contexts per spec).

### 2. Error model

Errors are M values per spec, not Rust panics or Prolog throws. They propagate automatically through evaluation; `try` is the only point that observes them.

**Decision:** internal `Result<Value, MError>` in Rust, internal predicate failure plus an explicit error term in Prolog. Wrap to a Value at `try` boundaries.

- Rust: `eval` returns `Result<Value, MError>`. Operators short-circuit on error in either operand. `try` is the only thing that catches `MError` and either returns a "with-value" record (success) or a "with-error" record (failure).
- Prolog: `eval(Ast, Env, Value)` succeeds with a Value or fails (Prolog-level failure); on failure, an associated error term is recorded. `try` predicate catches the failure and constructs the appropriate result record. (Alternative: thread a `Result(Value)` term through eval everywhere — TBD which is more idiomatic for Prolog.)

The user-facing M `error` value has shape `[HasError = true, Error = ...]` versus success `[HasError = false, Value = ...]`. The Prolog and Rust constructions of these records must agree byte-for-byte.

### 3. Environment representation

Lazy bindings + closures + recursive `let` → persistent linked environments.

- A closure captures its enclosing environment by reference, not by copy.
- Mutual recursion in `let` works because all bindings are thunked references in the same env: when binding `b` is forced and references `a`, `a` is looked up in the same env that contains `b`.
- The `@` scoping operator forces a lookup that includes the function's own binding (for recursion).

**Decision:**

- Rust: `type Env = Rc<EnvNode>` where `EnvNode { bindings: HashMap<String, Value>, parent: Option<Env> }`. Lookup walks the chain. `extend` produces a new `EnvNode` whose `parent` points at the current one. Sharing is free; new envs are tiny.
- Prolog: env as a list of `frame(Bindings)` where `Bindings` is an assoc list (`library(assoc)`) or pairs list. `lookup/3` walks the list. `extend/3` prepends a new frame. Persistence is automatic in Prolog (no destructive updates).

### 4. Value representation

Tagged enum / Prolog term tree. Both sides agree on the variant set:

| M kind | Rust | Prolog |
|---|---|---|
| null | `Value::Null` | `null` |
| logical | `Value::Logical(bool)` | `bool(true)` / `bool(false)` |
| number | `Value::Number(f64)` | `num(F)` (always a float) |
| text | `Value::Text(String)` | `text(Cs)` (chars list) |
| date / datetime / etc. | `Value::Date(NaiveDate)` etc. | `date(Y,M,D)` etc. |
| duration | `Value::Duration(...)` | `duration(...)` |
| binary | `Value::Binary(Vec<u8>)` | `binary(Bytes)` |
| list | `Value::List(Vec<Value>)` | `list(Items)` |
| record | `Value::Record(Vec<(String,Value)>)` ordered | `record(Pairs)` ordered |
| table | `Value::Table(arrow::RecordBatch)` | `table(Cols)` (list-of-records fallback in Prolog — slow but correct) |
| function | `Value::Function(Closure)` | `closure(Params, Body, Env)` |
| type | `Value::Type(TypeRep)` | `type_value(Repr)` |
| error | not a Value variant — separate `MError` | not a value term — predicate failure + recorded reason |
| thunk | `Value::Thunk(...)` | `thunk(Expr, Env)` |

Records preserve insertion order per spec (M's records are ordered, not sets).

### 5. Number representation

M's `number` is f64-equivalent in practice. Microsoft uses double-precision floats with IEEE 754 semantics: NaN, infinity, signed zero, denormals.

**Decision:** f64 in Rust, *always force-to-float* in Prolog (`F is float(N)` after any arithmetic) so `0xff` in Rust gives `255.0` and in Prolog also gives `255.0`, not Prolog's native bigint `255`. Edge cases (very large integers losing precision, NaN ordering, `1.0 / 0.0`) match Microsoft via goldens later — we don't invent semantics here.

## Slicing plan

Each evaluator slice ships with the Rust implementation, the Prolog mirror, and an extension to a `diff_eval.sh` harness. Both sides agree slice-by-slice; the differential never regresses.

| Slice | Scope |
|---|---|
| eval-1 | Literals, identifier lookup, unary, all binary operators, `if`/`then`/`else`, `let`/`in` with lazy mutual-recursive bindings |
| eval-2 | Function literals (closures), invocation, `each` desugaring, the `@` self-reference operator (needs parser addition) |
| eval-3 | Lists (with ranges), records, field access (incl. optional `?`), item access (incl. optional `?`), implicit `[name]` access on `_` |
| eval-4 | `try expr`, `try expr otherwise fallback`, `error expr` |
| eval-5 | Type system: `type X` constructs type values, `as` runtime conformance check, `is` runtime test, primitive type compatibility |
| eval-6 | Starter stdlib: `Number.From`, `Text.*`, `List.Sum/Count/Min/Max`, `Record.Field/FieldNames`, `Logical.*` — bound in the root env. Intrinsic constructors `#date`/`#datetime`/`#duration`/`#nan`/`#infinity` etc. land here too. |
| eval-7 | Arrow-backed tables: `#table` constructor, `Table.*` functions, value↔Arrow conversions, Parquet IO at the CLI shell. This is where Rust and Prolog stop tracking each other — Prolog falls back to list-of-records for tables, Rust uses Arrow record batches. The differential harness uses small enough tables that both can run; production-shape Parquet inputs are Rust-only. |
| eval-8 | ODBC support: `Odbc.DataSource(conn, opts)` returning a navigation table with lazy `Data` columns; `Odbc.Query(conn, sql, opts)` for direct SQL. Built on top of the `IoHost` trait. CLI shell implements via `odbc-api`; WASM shell omits the names from the intrinsic env. ODBC type → M type mapping lives here. Prolog companion does not implement these — they're shell-side IO and there's no Prolog ODBC story worth pursuing. |

Stdlib coverage past slice 6 is corpus-driven — the function frequency table from the user's real query corpus (per `05-open-questions.md`) decides what lands in eval-7+.

## Parser prerequisite

The `@` self-reference operator is in the spec keyword list but not yet wired into the parser. eval-2 needs it. Add as a prefix in `parse_primary`: `@<identifier>` produces an `Identifier` reference that the evaluator interprets as "look up in the recursive scope" (most easily: same as a regular identifier — the recursive scope is just the current env, since closures capture themselves by being thunks in the same env they were bound to).

## Testing strategy

Three layers:

1. **Rust unit tests** — fast feedback, comprehensive per slice. Each slice's evaluator has its own test module.

2. **Prolog–Rust differential** — `tools/grammar-fuzz/diff_eval.sh` runs each test expression through both evaluators, compares canonical value output. Same shape as `diff_parser.sh`. Catches divergence in evaluation semantics. Limited to expressions that don't involve large tables or stdlib functions present in only one side.

3. **Microsoft M oracle goldens** — manual Windows runs producing Parquet, committed to repo per `04-test-harness.md`. CI diffs `mrsflow` output against committed goldens. Becomes the authoritative test once stdlib + table support land in eval-6/7. Until then, the Prolog companion fills the oracle role for pure expressions.

## Canonical value format for the differential

Both implementations need to print values in the same format for `diff_eval.sh` to work. Reuse the S-expression style from `diff_parser.sh`:

| Value | S-expression |
|---|---|
| `null` | `(null)` |
| `true` / `false` | `(bool true)` / `(bool false)` |
| number `42` | `(num 42)` (or `42.0` — TBD how floats render) |
| text `"hi"` | `(text "hi")` (escaped per parser format) |
| list `{1, 2}` | `(list ((num 1) (num 2)))` |
| record `[a=1, b=2]` | `(record (("a" (num 1)) ("b" (num 2))))` |
| function | `(function ...)` (no canonical interior — function equality is reference equality per spec) |
| error | not a value to print — the runner prints `(error "<reason>")` to stderr |

Numeric rendering is the one spot to think about — `42` vs `42.0` could diverge between Rust's `f64::to_string()` and Prolog's float printer. Solution: pick a canonical format (always trailing `.0` for whole numbers, or always shortest representation) and have both sides emit it.

## Open questions deferred

- **Memoisation on records.** Spec says record field forcing is memoised — but is it memoised across record-value clones? If `[a = expensive()]` is bound to `r` and `r2`, does `r[a]` and `r2[a]` share the memo? Probably yes (both are the same record value, shared via `Rc`). Worth confirming when slice 3 lands.

- **`@` semantics.** Per spec, `@functionName` references the enclosing function in a recursive context. With closures-as-thunks-in-same-env, the regular identifier lookup probably suffices. Confirm at slice 2.

- **Type compatibility lattice.** `type any` vs `type number` vs `type nullable number` — slice 5 needs a small subset of M's compatibility rules. Defer the full lattice (record subtyping, table compatibility) until corpus drives it.

- **Concurrent evaluation.** mrsflow-core is sync. If a future shell wants to parallelise across rows or columns, that's the shell's problem (use `rayon` or similar at the IO boundary). The evaluator stays single-threaded.

- **Recursive `Rc<RefCell>` and the borrow checker.** Closures-in-env-with-thunks-that-reference-the-env can hit awkward borrow patterns. If `RefCell` becomes painful, switch to `Arc<Mutex>` (still single-threaded but avoids `RefCell`'s panic-on-conflict) or to an arena allocator. Decide when slice 1 needs it.

- **ODBC connection pooling.** Default in eval-8 is open-per-call. If a real workload shows the same DSN being hit repeatedly within one M evaluation (likely with `Odbc.DataSource` followed by multiple `Source{[Name=…]}[Data]` accesses), add a per-evaluator-run connection cache keyed on the connection string. Implement when corpus shows it matters, not preemptively.

- **ODBC type mapping.** SQL types → M types is a mini type-coercion swamp. First pass: integers/floats → `Number`, all string types → `Text`, `BOOLEAN` → `Logical`, `DATE` → `Date`, `TIMESTAMP` → `Datetime`, `BYTEA`/`BLOB` → `Binary`, NULL → `Null`. Edge cases (precision-sensitive `DECIMAL`, time zones on `TIMESTAMPTZ`, vendor-specific types) handled as the corpus surfaces them.
