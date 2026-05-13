# vendor/

Local copies of upstream crates with small patches that aren't (yet) in their
released versions. Wired in via `[patch.crates-io]` in the workspace root
`Cargo.toml`.

## odbc-api 13.1.0

**Currently carries no functional patches — kept as a vendored copy so we
can intervene quickly if a future driver-quirk surfaces.**

### History: the indicator-panic patch (reverted)

An earlier version of this README documented a one-line patch to
`Indicator::from_isize` that mapped unrecognised negative SQLLEN values
to `Null` instead of panicking. The intent was to mask DBISAM's broken
indicator behaviour and let the columnar fast path keep running.

That was wrong: DBISAM's bad indicator bytes apply to *valid row data*,
not just NULL sentinels. With the patch in place, q13/q14/q15 silently
returned 143 real rows followed by ~141 fake `Null` rows on a 284-row
table — content corruption that matched Excel's row count and so passed
casual inspection.

The patch was reverted in commit `72decce`. The upstream `expect` now
panics on the offending values; `mrsflow-cli`'s `odbc_query_impl` catches
the panic via `catch_unwind`, blocklists the connection string, and
falls back to `SQLGetData` row-at-a-time, which decodes correctly.

### Why the panic happens at all

DBISAM 4.39 (the Exportmaster driver, dated 2014-05-15, discontinued
product) was built when 64-bit ODBC drivers commonly truncated `SQLLEN`
slots to 32 bits. The driver IS x64; both `mrsflow.exe` and `dbodbc.dll`
are x64. But the driver writes 32-bit values into 64-bit slots and the
high 32 bits are uninitialized memory. `odbc-api`'s `from_isize` sees a
nonsense isize (either huge positive or arbitrary negative) and panics
on the `try_into::<usize>` for negatives.

ODBCINST.INI advertises:

    DriverODBCVer    : 03.00
    APILevel         : 1
    SQLLevel         : 0   (minimal SQL)

There's no driver-side flag or connection-string knob that fixes this.
ODBC 3 specifies 64-bit `SQLLEN` on 64-bit platforms; DBISAM 4.x predates
the fix. Investigation (committed in this README) covered:

- DSN registry entries (HKLM\SOFTWARE\ODBC\ODBC.INI\Exportmaster):
  no length-handling option.
- Driver registry (HKLM\SOFTWARE\ODBC\ODBCINST.INI\DBISAM 4 ODBC Driver):
  no length-handling option.
- The driver DLL file properties: ProductVersion 4.39, Build 1, dated
  2014-05-15. Elevate Software replaced DBISAM with ElevateDB; no
  active development on DBISAM.

### What the runtime cost is now

One columnar attempt per session per connection string. The first query
panics inside `as_text_view().get(row)`, `catch_unwind` catches, the
connection string lands in `columnar_blocklist`, and we fall back to
row-at-a-time. Subsequent queries on the same connection skip the
columnar attempt entirely. The default panic hook is swapped out during
the attempt so the recoverable panic doesn't dump a trace to stderr.

A possible future micro-optimisation: probe `SQL_DRIVER_NAME` via
`SQLGetInfo` at connect time and pre-blocklist if it contains "DBISAM".
Not built yet — `odbc-api` 13.1.0 doesn't expose `SQLGetInfo` in its
safe surface, so this would need unsafe FFI or an upstream PR.

## Updating

When bumping to a newer odbc-api: replace `vendor/odbc-api/` with the new
release tarball (or `cargo vendor` snapshot). No patches to re-apply
currently. Run `cargo test` and at minimum
`examples/nisa/Country.pq` end-to-end against the live DSN to confirm
the panic-catch path still works.
