# vendor/

Local copies of upstream crates with small patches that aren't (yet) in their
released versions. Wired in via `[patch.crates-io]` in the workspace root
`Cargo.toml`.

## odbc-api 13.1.0

One change, in `src/buffers/indicator.rs`:

```rust
pub fn from_isize(indicator: isize) -> Self {
    match indicator {
        NULL_DATA => Indicator::Null,
        NO_TOTAL => Indicator::NoTotal,
        n if n < 0 => Indicator::Null,           // <- added
        other => Indicator::Length(other as usize),
    }
}
```

Upstream panics via `try_into().expect(...)` on any negative SQLLEN value
that isn't one of the two named sentinels. DBISAM (the Exportmaster
driver) returns other negative values — probably a 32-bit SQLLEN
mis-sign-extended into 64-bit — and the panic propagates up through the
columnar text/bin buffers' `Indicator::from_isize` callers.

Treating unrecognised negatives as `Null` is the conservative recovery:
the row's data is "we don't have a length we trust", so present it as
null rather than crash the process. Once we've seen a well-formed value
again the rest of the column works.

## Updating

When bumping to a newer odbc-api: replace `vendor/odbc-api/` with the new
release tarball (or `cargo vendor` snapshot), then re-apply the patch
above. Run `cargo test` and `examples/nisa/Country.pq` end-to-end to
confirm.
