# Not-yet-supported stdlib options

Auto-generated punch list of `NotImplemented(...)` / `"not yet supported"`
markers across the stdlib. Regenerate with:

```sh
grep -rohE '(NotImplemented\("[^"]+"|"[A-Z][a-zA-Z.]+: [^"]*not yet (supported|implemented)")' \
    mrsflow-core/src mrsflow-cli/src | sort -u
```

Markers count was **71** at the time of writing. Bucketed by shape so
similar gaps get filled together.

## Format-string overloads (4)

The single-arg form works; the two-arg `ToText(value, format)` /
three-arg `ToText(value, format, culture)` forms reject everything.
Same pattern `Number.ToText` already handles after commit `8a1074e` —
accept `"G"` / `"g"` / empty as general (no-format equivalent),
error on others with the actual format string in the message.

- ✅ `DateTime.ToText`
- ✅ `DateTimeZone.ToText`
- ✅ `Duration.ToText`
- ✅ `Time.ToText`

## equationCriteria / comparisonCriteria — DONE

The biggest bucket. Functions take an optional record / function /
list that customises how two values are compared for equality or
ordering. Default behaviour (omitted criteria) uses primitive
equality / natural ordering. Implementing requires a callback path.

- ✅ `List.Contains` / `ContainsAll` / `ContainsAny`
- ✅ `List.Difference` / `Intersect` / `Union`
- ✅ `List.IsDistinct` / `Mode` / `Modes`
- ✅ `List.PositionOf` / `PositionOfAny`
- ✅ `List.Sort` (comparisonCriteria, not equationCriteria)
- ✅ `Table.Contains` / `ContainsAll` / `ContainsAny`
- ✅ `Table.Distinct` / `IsDistinct`
- ✅ `Table.Group` (comparisonCriteria + groupKind)
- ✅ `Table.PositionOf` / `PositionOfAny`
- ✅ `Table.RemoveMatchingRows` / `ReplaceMatchingRows`
- ✅ `Value.Equals`

## Predicate-form arguments — DONE

The Nth-item form (`(table, 5)`) works; the predicate form
(`(table, each [x] > 10)` — take-while or skip-while) is unimplemented.

- ✅ `List.FirstN` / `LastN` / `Skip` / `RemoveFirstN` / `RemoveLastN`
- ✅ `Table.FirstN` / `LastN` / `Skip`

## quoteStyle / startAtEnd flags — 2 left (positions/ranges-from-end)

Splitter / Combiner options. `quoteStyle` is the same enum
`Csv.Document` already honours (None / Csv); `startAtEnd` reverses
the scan direction.

The two unticked entries below need empirical PQ testing to confirm
positions-from-end semantics — left for a later slice with Oracle
support.

- ✅ `Splitter.SplitTextByDelimiter`
- ✅ `Splitter.SplitTextByAnyDelimiter`
- ✅ `Splitter.SplitTextByEachDelimiter` (both flags)
- ✅ `Splitter.SplitTextByLengths`
- `Splitter.SplitTextByPositions`
- `Splitter.SplitTextByRanges`
- ✅ `Splitter.SplitTextByWhitespace`
- ✅ `Combiner.CombineTextByDelimiter`

## missingField option — DONE

PQ enum: `MissingField.Error` (default), `MissingField.Ignore`,
`MissingField.UseNull`. Add as numeric constants then dispatch.

- ✅ `Record.RemoveFields` / `RenameFields` / `ReorderFields`
  / `SelectFields` / `TransformFields`

## occurrence — DONE

Which match to return (first / last / all). Default first.

- ✅ `List.PositionOf` / `PositionOfAny`
- ✅ `Table.PositionOf` / `PositionOfAny`
- ✅ `Text.PositionOfAny`

## Other one-offs (~16)

Smaller per-function options, lower aggregate impact.

- `Combiner.CombineTextByLengths` / `ByPositions` / `ByRanges` (template)
- ✅ `Table.AddRankColumn` (options.RankKind — Competition/Ordinal/Dense; Modified unsupported)
- ✅ `Table.FromList` (default arg)
- `Table.FromPartitions` (columnInfo)
- ✅ `Table.FromValue` (options.Name)
- ✅ `Table.Join` (composite keys)
- ✅ `Table.Profile` (additionalAggregates)
- ✅ `Table.SplitColumn` (default + extraValues)
- ✅ `Table.TransformColumnNames` (options accepted-and-ignored)
- ✅ `Value.FromText` (culture — accepted-and-ignored)
- ✅ `List.Percentile` (options.PercentileMode — ExcelInc only)
- ✅ `List.Random` (seed)
- `unsupported cell type` (internal — Arrow encode path)

## How to use this list

Pick a bucket. Implement one shape; the others in the bucket
usually fall out from the same helper. Re-run the grep and prune
this file when done.

Oracle (`Oracle/cases/q*.m`) is the best signal for which gaps
actually bite real queries — if a `q.mrsflow.out` says
`ERROR: ... not yet supported`, that's a corpus-driven priority.
