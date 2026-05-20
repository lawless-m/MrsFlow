# Contributing to MrsFlow

This is a single-author pre-v1 tool, but the workflow is written down so
that (a) future-me can pick it back up and (b) anyone reading the repo
can see how the pieces fit. The house rules are in
[`../CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md): work is judged on
whether it makes the software better.

## The golden rule: the oracle decides

mrsflow's correctness target is **"whatever real Power Query does."**
Not the documentation — the documentation is wrong often enough that
trusting it has burned us repeatedly (see
[`COMPATIBILITY.md`](COMPATIBILITY.md)). When you implement or change a
function, you prove it by adding an Oracle test case that runs the same
M expression through Excel and through mrsflow and diffs the output.

You need Windows + Excel with Power Query to *generate* oracle outputs.
If you don't have that, you can still write the Rust and the q-case;
just flag in the PR that the `.excel.out` needs regenerating.

## Adding a stdlib function

Worked example: adding `Geometry.ToWellKnownText`.

### 1. Find or create the module

stdlib lives in `mrsflow-core/src/eval/stdlib/`, one file per family
(or cluster of related families). `geo.rs` holds the `Geography.*` /
`Geometry.*` functions. If your family has no home yet, create
`myfamily.rs` and register it in `mod.rs`:

```rust
// in stdlib/mod.rs
mod myfamily;                       // 1. declare the module
// …then in builtin_bindings():
        myfamily::bindings(),       // 2. add to the binding list
```

### 2. Write the binding table

Each module exposes `pub(super) fn bindings()` returning
`Vec<(name, params, BuiltinFn)>`. Use the `one`/`two`/`three` helpers
for fixed arity, or a full `vec![Param { … }]` for optional args:

```rust
pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Geometry.ToWellKnownText", one("value"), geometry_to_wkt),
        (
            "GeometryPoint.From",
            vec![
                Param { name: "x".into(),    optional: false, type_annotation: None },
                Param { name: "y".into(),    optional: false, type_annotation: None },
                Param { name: "z".into(),    optional: true,  type_annotation: None },
                Param { name: "srid".into(), optional: true,  type_annotation: None },
            ],
            geometry_point_from,
        ),
    ]
}
```

### 3. Write the handler

A `BuiltinFn` is `fn(&[Value], &dyn IoHost) -> Result<Value, MError>`.
Pure functions ignore the host. Use `common::expect_*` to validate
inputs and `common::type_mismatch` for the standard error:

```rust
fn geometry_to_wkt(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let (x, y) = expect_point_fields(&args[0], "X", "Y")?;
    Ok(Value::Text(format!("POINT({x} {y})")))
}
```

**Match PQ's error wording.** When PQ rejects an input, it uses fixed
phrases like `"We cannot convert the value null to type Text."` Mirror
them exactly — the oracle diffs error messages too. Several commits in
the history are nothing but wording fixes
(`Variable.Value`, `Value.Versions`, `Value.Expression`).

For constants (enum tokens, sentinels), don't write a handler — extend
the env directly in `mod.rs`:

```rust
for (name, n) in [
    ("AccessControlKind.Deny",  0.0),
    ("AccessControlKind.Allow", 1.0),
] {
    env = env.extend(name.to_string(), Value::Number(n));
}
```

### 4. Build with the right features

```bash
cargo build --release --bin mrsflow --features odbc
```

The `odbc` feature matters: the Oracle catalog references an ODBC DSN,
and a non-odbc build makes three unrelated cases fail with
"built without ODBC support."

### 5. Add an Oracle q-case

`Oracle/Oracle.m` is one big M `let` whose `cases` list holds
`SafeSerialize("qN", () => <expr>)` rows. Append yours with the next
number (grep for the last `q\d+`):

```m
SafeSerialize("q1514", () =>
    Geometry.ToWellKnownText(GeometryPoint.From(10, 20))),
```

`SafeSerialize` wraps the expression in `try` and renders either the
value (via `Json.FromValue`) or `ERROR: <message>`. **Type values and
function values don't survive `Json.FromValue`** — if your function
returns one, project to a scalar in the q-case (e.g.
`Type.IsNullable(...)`, `Value.Is(..., type function)`, or `[Kind]`).

### 6. Run the oracle pipeline

```bash
pwsh Oracle/QueryOracle.ps1        # refresh Excel, dump cases/qN.excel.out
pwsh Oracle/capture_mrsflow.ps1    # run mrsflow, dump cases/qN.mrsflow.out
pwsh Oracle/diff.ps1               # compare, print Summary + DIFF list
```

`diff.ps1` normalises cosmetic noise (number formatting, timestamps,
environment paths) before comparing, so a visible byte difference can
still be a MATCH. The baseline is **3 DIFFs** (documented in
[`COMPATIBILITY.md`](COMPATIBILITY.md)); if you see more, you've
regressed something.

### 7. Regenerate coverage and commit

```bash
pwsh Oracle/coverage/extract_cases.ps1   # write per-case cases/qN.m
pwsh Oracle/coverage/gen_status.ps1      # cases_status.tsv + case_names.tsv
pwsh Oracle/coverage/render.ps1          # COVERAGE.md + Function_Families.md
```

Commit the Rust change, the q-case, both `.out` files, the per-case
`.m`, and the regenerated coverage artefacts together.

## Gotchas the oracle taught us

- **Excel caches compiled catalog rows.** If you edit a q-case and Excel
  keeps emitting the old result, kill the Excel process before
  re-running `QueryOracle.ps1`. Adding a *new* q-case always works.
- **`type type` is a parser quirk** — mrsflow's tokeniser treats `type`
  as reserved, so `Value.Is(x, type type)` won't parse. Use `Type.Type`.
- **Some functions are Excel-compile-rejected** outside a connector
  context (`Value.Firewall`, `Value.NativeQuery`,
  `Tables.GetRelationships`, `IdentityProvider.Default`). Excel emits
  nothing for them; the diff tool's empty-Excel rule still counts a
  non-empty mrsflow result as MATCH, but you can't assert a specific
  value.
- **Clock-tight functions flake.** `DateTime.IsInCurrentSecond` and the
  `IsInNext/PreviousMinute/Second` family depend on Excel and mrsflow
  evaluating "now" within the same second — they're left untested.

## The coverage scanner

`gen_status.ps1` attributes a q-case to a function by substring-matching
the function name followed by `(`, `.`, `,`, `)`, `}`, `[`, space, or
end-of-line in the case's `.m` source. If a function you tested shows
as untested on the dashboard, it's usually this heuristic missing a
delimiter — widen the trigger set rather than contorting the q-case.
