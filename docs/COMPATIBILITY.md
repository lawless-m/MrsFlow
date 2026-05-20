# Compatibility & the misleading-docs findings

mrsflow targets bug-for-bug agreement with the real Power Query engine,
not with Microsoft's documentation. This file records (1) the handful of
places mrsflow still diverges, and (2) the larger set of cases where the
oracle proved the *documentation* wrong and we matched the engine
instead.

## Current divergences (the 3 baseline DIFFs)

These are the only Oracle cases that don't match Excel, and none is a
silent wrong answer:

| Case  | What                          | Why it differs                                                                                          |
| ----- | ----------------------------- | ------------------------------------------------------------------------------------------------------- |
| q1165 | `#shared` catalogue dump      | The list of available functions differs by construction — Excel exposes connectors mrsflow doesn't, and vice-versa. Not fixable without implementing every connector. |
| q1167 | `File.Contents` on a relative path | Excel rejects with "The supplied file path must be a valid absolute path."; mrsflow accepts relative paths (the coverage dashboard's own loader depends on this). Error *wording* differs; both refuse the same bad paths otherwise. |
| q1179 | `BinaryFormat.Group` arity    | Excel's `Group` takes 2–4 args (format, count, type, transform); mrsflow's takes a list of formats. Different API shape; reconciling means a redesign, and the list-form is what the corpus uses. |

Everything else — 1,522 of 1,525 cases — matches byte-for-byte after
`diff.ps1`'s cosmetic normalisation (number formatting, timestamps,
environment paths).

## Where the docs lied and the engine won

Each of these was a "the documentation says X, the engine does Y, we did
Y" correction the oracle forced. They're the strongest argument for
differential testing over spec-reading.

### Enum ordinals don't match the docs

The documented ordinals for several enum families are simply wrong, or
the families overload ordinals across contexts:

- **`Compression`**: `None = -1`, `GZip = 0`, `Deflate = 1`, `Brotli = 3`
  (not the sequential 0,1,2,3 you'd guess).
- **`GroupKind`**: `Local = 0`, `Global = 1` (docs imply the reverse).
- **`RankKind`**: `Competition = 0`, `Dense = 1`, `Ordinal = 2` (the
  Dense/Ordinal pair is swapped from intuition).
- **`Precision`**: `Double = 0`, `Decimal = 1`.
- **`Occurrence`**: overloaded — `First = Optional = 0`, `Last = Required
  = 1`, `All = Repeating = 2`. The same ordinal slot serves two
  different documented constant names.
- **`RoundingMode`**, **`ExtraValues`**, **`PercentileMode`**,
  **`ByteOrder`**, **`BinaryOccurrence`**, **`BufferMode`** — all needed
  ordinal corrections discovered against the oracle.

### Constants that are text, not numbers

- **`WebMethod.*`** — `GET`/`POST`/… are the uppercase *verb text*, not
  numeric ordinals.
- **`ODataOmitValues.Nulls`** — the lowercase text `"nulls"`.
- **`TimeZone.Current`** — returns the host's Windows timezone display
  name (e.g. `"GMT Standard Time"`) as **text**, not a duration as the
  shape would suggest.
- **`Culture.Current`** — the BCP-47 locale string (`"en-GB"`), and
  Excel auto-invokes the parameter-less function when referenced bare.

### Enum `.Type` constants are the type-of-types

`BinaryEncoding.Type`, `Day.Type`, `JoinKind.Type`, … are **not**
aliases for the underlying numeric representation. They're `type type`
values. `Type.Is(0, BinaryEncoding.Type)` raises
"We cannot convert the value 0 to type Type." in Excel — it doesn't
return `true`. (`Byte.Type` is the exception: a genuine numeric
subtype.)

### Numeric / binary precision

- **`Single.From(3.14)`** returns `3.140000104904175` — Excel rounds
  through a 32-bit float and back. A naive identity returns `3.14` and
  is wrong.
- **`BinaryFormat.UnsignedInteger16`** and the rest default to
  **big-endian** (network byte order), not little-endian.
- **`BinaryFormat.7BitEncodedSignedInteger`** uses **zigzag** encoding
  (`(n >> 1) ^ -(n & 1)`), not a two's-complement reinterpretation.
- **Time/Datetime fractional seconds** serialise to JSON at .NET
  DateTime tick resolution — 7 digits, `14:59:59.9999999`, not the
  9-digit nanoseconds chrono prints.

### Return-shape surprises

- **`Value.Lineage`** returns a record `[Name="", Value=null, To={}]`;
  **`Value.Traits`** returns an empty *list* `{}`. The two are easy to
  get backwards.
- **`Binary.InferContentType`** always returns a record with at least a
  null `Content.Type` field — never a bare null.
- **`Table.PartitionKey`** → `null`, **`Table.PartitionValues`** →
  `[{}]` (a single empty record), **`Type.TablePartitionKey`** → `null`
  for unpartitioned tables. Three different "nothing here" sentinels.
- **`Type.Facets`** returns a fixed 10-field record-of-nulls, not an
  empty record.
- **`Lines.ToBinary`** terminates *every* line with the separator,
  including the last.
- **`Uri.BuildQueryString`** omits the leading `?`.
- **`Table.CombineColumnsToRecord`** places the new record column at the
  position of the *first* source column, not at the end.
- **`Html.Table`** requires an explicit `RowSelector` option once more
  than one column is specified — otherwise it errors rather than
  defaulting to one row.

### `List.Alternate` cycle semantics

The MS docs example for `List.Alternate({1..7}, 1)` implies `[1,3,5]`.
Excel actually returns `[2,3,4,5,6,7]` — the default `repeatInterval` is
unbounded (drop `count` items once, keep the rest), and the cycle length
is `count + repeatInterval`, not `repeatInterval` alone.

## How these were found

All of the above came out of the `Oracle/` differential harness: write
the M expression, run it through both engines, diff. The pattern in the
git history is consistent — a `test(oracle):` commit that parks a
mismatch, then a `fix(...)` commit that corrects mrsflow to match the
engine and un-parks the case. See [`CHANGELOG.md`](CHANGELOG.md).
