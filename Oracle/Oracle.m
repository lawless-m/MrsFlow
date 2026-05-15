// Oracle.m — one Power Query, one worksheet, two columns: Q | Result.
//
// Workflow:
//   1. Open Oracle.xlsx. Replace the existing "Catalog" query (or
//      create one) with this file's contents — paste into Advanced
//      Editor.
//   2. Load To… → Table on a single worksheet.
//   3. QueryOracle.ps1 dumps that one sheet, no per-test tab juggling.
//
// Each row is one test. Bodies are wrapped in `SafeSerialize("qN",
// () => <body>)` so that:
//   - errors (cycles, type mismatches, missing DSN, …) come back as
//     "ERROR: <message>" instead of halting the whole catalog,
//   - scalars (text/number/logical/null) serialize as human-readable
//     text,
//   - everything else (tables, records, lists, binary, dates) goes
//     through Json.FromValue → UTF-8 text. JSON divergence between
//     PQ and mrsflow would manifest as every table-returning row
//     failing identically — very loud, not a silent foot-gun.
//
// Adding a test = append one SafeSerialize line to `cases`.
//
// The same bodies live in Oracle/cases/qN.m for mrsflow-side
// regeneration; keep them in sync.

let
    Oracle.Serialize = (v as any) as text =>
        if v = null then "null"
        else if v is text then v
        // Text.From over Number.ToText(v, "G", "en-US") — same result
        // for integer-valued and decimal numbers, doesn't depend on the
        // format-string overload of Number.ToText that mrsflow doesn't
        // implement yet.
        else if v is number then Text.From(v)
        else if v is logical then (if v then "true" else "false")
        else Text.FromBinary(Json.FromValue(v), TextEncoding.Utf8),

    SafeSerialize = (label as text, expr as function) as record =>
        let
            r = try expr()
        in
            if r[HasError]
                then [Q = label, Result = "ERROR: " & r[Error][Message]]
                else [Q = label, Result = Oracle.Serialize(r[Value])],

    cases = {
        // q1, q2: cycle detection cases ([X=X][X] and [a=b,b=a][a]).
        // Power Query rejects these at *compile time* — `Name 'X' doesn't
        // exist` — so they'd block the entire Catalog from loading rather
        // than fail one row. mrsflow detects them at *evaluate time*
        // (thunk re-entry). Different mechanism, same outcome on each
        // engine. The qN.m files for q1/q2 stay under Oracle/cases/ for
        // mrsflow-side regression; just not in the Catalog.

        // q3: Date.ToText dd-MMM-yy.
        SafeSerialize("q3", () =>
            Date.ToText(#date(2026, 6, 15), "dd-MMM-yy")),

        // q4: Date.ToText long English form — locale-dependent in PQ?
        SafeSerialize("q4", () =>
            Date.ToText(#date(2026, 1, 5), "dddd, MMMM d, yyyy")),

        // q5: Date.ToText 2-digit year and zero-padded MM/dd.
        SafeSerialize("q5", () =>
            Date.ToText(#date(2026, 6, 5), "yy.MM.dd")),

        // q6: Date.ToText unpadded M/d.
        SafeSerialize("q6", () =>
            Date.ToText(#date(2026, 6, 5), "M/d")),

        // q7: Table.PromoteHeaders with PromoteAllScalars on heterogeneous
        //     scalar header row.
        SafeSerialize("q7", () =>
            Table.PromoteHeaders(
                #table({"A","B"}, {{1.5, true}, {"x", "y"}}),
                [PromoteAllScalars=true])),

        // q8: Text.ToBinary with the BinaryEncoding.Base64 quirk —
        //     mrsflow keeps strict and errors; what does PQ do?
        SafeSerialize("q8", () =>
            Text.FromBinary(Text.ToBinary("hello", BinaryEncoding.Base64))),

        // q9: Binary.ToText Base64 of UTF-8 "hello".
        SafeSerialize("q9", () =>
            Binary.ToText(Text.ToBinary("hello"), BinaryEncoding.Base64)),

        // q10: Csv.Document QuoteStyle.None preserves literal quotes.
        SafeSerialize("q10", () =>
            Csv.Document(
                Text.ToBinary("a,""b,c"",d"),
                [Delimiter=",", QuoteStyle=QuoteStyle.None])),

        // q11: Folder.Contents Attributes record on a known directory.
        //      Linux mrsflow vs Windows PQ exposes a different attribute
        //      set — divergence here is expected and informative.
        SafeSerialize("q11", () =>
            Folder.Contents("C:\Windows\System32"){0}[Attributes]),

        // q12: Excel.CurrentWorkbook from inside the host workbook.
        //      Includes every named cell + ListObject; the Catalog
        //      table itself appears here on refresh #2 onwards.
        SafeSerialize("q12", () => Excel.CurrentWorkbook()),

        // q13: ODBC fold — column projection should push down to
        //      `SELECT RITerritoryCode, RITerritoryDesc FROM RIGeographic`.
        //      Semantics: 284 rows × 2 columns. Verified byte-for-byte
        //      against Excel.
        SafeSerialize("q13", () =>
            Table.SelectColumns(
                Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true])
                    {[Name="NISAINT_CS",Kind="Database"]}[Data]
                    {[Name="RIGeographic",Kind="Table"]}[Data],
                {"RITerritoryCode", "RITerritoryDesc"})),

        // q14: ODBC fold — row predicate should push down to
        //      `SELECT * FROM RIGeographic WHERE RITerritoryCode = 'GB'`.
        SafeSerialize("q14", () =>
            Table.SelectRows(
                Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true])
                    {[Name="NISAINT_CS",Kind="Database"]}[Data]
                    {[Name="RIGeographic",Kind="Table"]}[Data],
                each [RITerritoryCode] = "GB")),

        // q15: ODBC fold — combined projection + predicate.
        //      `SELECT RITerritoryDesc FROM RIGeographic
        //           WHERE RITerritoryCode = 'GB'`.
        SafeSerialize("q15", () =>
            Table.SelectColumns(
                Table.SelectRows(
                    Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true])
                        {[Name="NISAINT_CS",Kind="Database"]}[Data]
                        {[Name="RIGeographic",Kind="Table"]}[Data],
                    each [RITerritoryCode] = "GB"),
                {"RITerritoryDesc"})),

        // q16-q29: top-13 unexercised stdlib functions by corpus call
        // frequency. Each case is the smallest one-liner that
        // exercises the function's documented happy-path.

        // q16: Table.RenameColumns — bulk rename.
        SafeSerialize("q16", () =>
            Table.RenameColumns(
                #table({"A","B"}, {{1,2}}),
                {{"A","X"},{"B","Y"}})),

        // q17: Table.RemoveColumns — drop one.
        SafeSerialize("q17", () =>
            Table.RemoveColumns(
                #table({"A","B","C"}, {{1,2,3}}),
                {"B"})),

        // q18: Table.TransformColumnTypes — coerce text to Int64.
        SafeSerialize("q18", () =>
            Table.TransformColumnTypes(
                #table({"N"}, {{"42"}}),
                {{"N", Int64.Type}})),

        // q19: Table.AddColumn — derived column via `each`.
        SafeSerialize("q19", () =>
            Table.AddColumn(
                #table({"A"}, {{10}}),
                "B",
                each [A] * 2)),

        // q20: Text.Replace — substring substitution.
        SafeSerialize("q20", () =>
            Text.Replace("hello world", "world", "there")),

        // q21: List.Transform — map a function over a list.
        SafeSerialize("q21", () =>
            List.Transform({1,2,3}, each _ * 10)),

        // q22: Table.ColumnNames — returns a list of text.
        SafeSerialize("q22", () =>
            Table.ColumnNames(#table({"A","B","C"}, {{1,2,3}}))),

        // q23: Json.Document — parse a JSON array literal.
        SafeSerialize("q23", () =>
            Json.Document("[1,2,3]")),

        // q24: Table.FromRows — rows + column names → table.
        SafeSerialize("q24", () =>
            Table.FromRows({{1,2},{3,4}}, {"A","B"})),

        // q25: Text.Contains — substring presence test.
        SafeSerialize("q25", () =>
            Text.Contains("hello world", "world")),

        // q26: Table.TransformColumns — per-column transform with type.
        SafeSerialize("q26", () =>
            Table.TransformColumns(
                #table({"A"}, {{5}}),
                {{"A", each _ + 1, Int64.Type}})),

        // q27: Text.From — convert a number to its text rendering.
        SafeSerialize("q27", () => Text.From(42)),

        // q28: Table.ExpandTableColumn — flatten a NestedJoin result.
        SafeSerialize("q28", () =>
            let
                a = #table({"k","x"}, {{1,"hello"}}),
                b = #table({"k","y"}, {{1,"world"}}),
                j = Table.NestedJoin(a, {"k"}, b, {"k"}, "right", JoinKind.LeftOuter)
            in
                Table.ExpandTableColumn(j, "right", {"y"})),

        // q29: Table.Combine — vertical concat of same-schema tables.
        SafeSerialize("q29", () =>
            Table.Combine({
                #table({"A"}, {{1}}),
                #table({"A"}, {{2}})})),

        // q30-q79: breadth pass across stdlib namespaces. Each case
        // is a one-liner that exercises one function's documented
        // happy path. Goal: lift catalog/stdlib ratio from ~4% to
        // ~10%+ without re-deriving novel tests every time.

        // --- Text.* ---
        SafeSerialize("q30", () => Text.Length("hello")),
        SafeSerialize("q31", () => Text.Upper("hello")),
        SafeSerialize("q32", () => Text.Lower("HELLO")),
        SafeSerialize("q33", () => Text.Start("hello world", 5)),
        SafeSerialize("q34", () => Text.End("hello world", 5)),
        SafeSerialize("q35", () => Text.Range("hello world", 6, 5)),
        SafeSerialize("q36", () => Text.Combine({"a", "b", "c"}, "-")),
        SafeSerialize("q37", () => Text.Split("a,b,c", ",")),
        SafeSerialize("q38", () => Text.Trim("  hello  ")),
        SafeSerialize("q39", () => Text.PadStart("42", 5, "0")),
        SafeSerialize("q40", () => Text.PadEnd("42", 5, "0")),
        SafeSerialize("q41", () => Text.Reverse("hello")),
        SafeSerialize("q42", () => Text.Repeat("ab", 3)),

        // --- Number.* ---
        SafeSerialize("q43", () => Number.From("3.14")),
        SafeSerialize("q44", () => Number.ToText(3.14)),
        SafeSerialize("q45", () => Number.Abs(-5)),
        SafeSerialize("q46", () => Number.Round(3.7)),
        SafeSerialize("q47", () => Number.RoundDown(3.7)),
        SafeSerialize("q48", () => Number.RoundUp(3.2)),
        SafeSerialize("q49", () => Number.IntegerDivide(17, 5)),
        SafeSerialize("q50", () => Number.Mod(17, 5)),
        SafeSerialize("q51", () => Number.Power(2, 10)),
        SafeSerialize("q52", () => Number.Sign(-7)),

        // --- List.* ---
        SafeSerialize("q53", () => List.Count({1,2,3,4,5})),
        SafeSerialize("q54", () => List.Sum({1,2,3,4,5})),
        SafeSerialize("q55", () => List.Average({1,2,3,4,5})),
        SafeSerialize("q56", () => List.Max({3,1,4,1,5,9,2,6})),
        SafeSerialize("q57", () => List.Min({3,1,4,1,5,9,2,6})),
        SafeSerialize("q58", () => List.First({1,2,3})),
        SafeSerialize("q59", () => List.Last({1,2,3})),
        SafeSerialize("q60", () => List.Reverse({1,2,3})),
        SafeSerialize("q61", () => List.Sort({3,1,4,1,5,9,2,6})),
        SafeSerialize("q62", () => List.Distinct({1,2,2,3,3,3})),
        SafeSerialize("q63", () => List.Skip({1,2,3,4,5}, 2)),
        SafeSerialize("q64", () => List.Range({1,2,3,4,5}, 1, 3)),
        SafeSerialize("q65", () => List.Repeat({1,2}, 3)),

        // --- Record.* ---
        SafeSerialize("q66", () => Record.FieldNames([a=1, b=2, c=3])),
        SafeSerialize("q67", () => Record.FieldValues([a=1, b=2, c=3])),
        SafeSerialize("q68", () => Record.HasFields([a=1, b=2], "a")),
        SafeSerialize("q69", () =>
            Record.RemoveFields([a=1, b=2, c=3], {"b"})),

        // --- Table.* (additional) ---
        SafeSerialize("q70", () => Table.RowCount(#table({"A"}, {{1},{2},{3}}))),
        SafeSerialize("q71", () => Table.ColumnCount(#table({"A","B","C"}, {{1,2,3}}))),
        SafeSerialize("q72", () => Table.FirstN(#table({"A"}, {{1},{2},{3},{4}}), 2)),
        SafeSerialize("q73", () => Table.Distinct(#table({"A"}, {{1},{2},{1},{3}}))),
        SafeSerialize("q74", () => Table.ReverseRows(#table({"A"}, {{1},{2},{3}}))),

        // --- Date / Time / Duration ---
        SafeSerialize("q75", () => Date.Year(#date(2026, 6, 15))),
        SafeSerialize("q76", () => Date.Month(#date(2026, 6, 15))),
        SafeSerialize("q77", () => Date.Day(#date(2026, 6, 15))),
        SafeSerialize("q78", () =>
            Date.AddDays(#date(2026, 1, 1), 10)),
        SafeSerialize("q79", () =>
            Duration.Days(#date(2026, 12, 31) - #date(2026, 1, 1))),

        // q80-q120: option-arg coverage for the work landed in the
        // /loop slices. Each case exercises one previously-rejected
        // option arg through the corpus path.

        // --- equationCriteria / comparisonCriteria (q80-q89) ---

        // q80: List.Contains with equationCriteria — case-insensitive match.
        SafeSerialize("q80", () =>
            List.Contains({"Hello","World"}, "HELLO",
                (a,b) => Text.Lower(a) = Text.Lower(b))),

        // q81: List.Distinct with equationCriteria — collapse case-variants.
        SafeSerialize("q81", () =>
            List.Distinct({"a","A","b","B","c"},
                (x,y) => Text.Lower(x) = Text.Lower(y))),

        // q82: List.Sort with custom comparisonCriteria — descending.
        SafeSerialize("q82", () =>
            List.Sort({3,1,4,1,5,9,2,6}, (a,b) => Value.Compare(b,a))),

        // q83: List.Intersect with case-insensitive equationCriteria.
        SafeSerialize("q83", () =>
            List.Intersect({{"A","B","C"}, {"a","b"}},
                (x,y) => Text.Lower(x) = Text.Lower(y))),

        // q84: List.PositionOf with equationCriteria.
        SafeSerialize("q84", () =>
            List.PositionOf({"X","Y","z"}, "Z", Occurrence.First,
                (a,b) => Text.Lower(a) = Text.Lower(b))),

        // q85: Table.Distinct with equationCriteria (row-vs-row).
        SafeSerialize("q85", () =>
            Table.Distinct(
                #table({"k","v"}, {{"a",1},{"A",2},{"b",3}}),
                (r1,r2) => Text.Lower(r1[k]) = Text.Lower(r2[k]))),

        // q86: Table.Contains with equationCriteria.
        SafeSerialize("q86", () =>
            Table.Contains(
                #table({"k"}, {{"alpha"},{"beta"}}),
                [k="ALPHA"],
                (r,n) => Text.Lower(r[k]) = Text.Lower(n[k]))),

        // q87: Value.Equals with equationCriteria callback.
        SafeSerialize("q87", () =>
            Value.Equals("Hello", "HELLO",
                (a,b) => Text.Lower(a) = Text.Lower(b))),

        // q88: List.Difference with equationCriteria.
        SafeSerialize("q88", () =>
            List.Difference({"A","B","C"}, {"a","c"},
                (x,y) => Text.Lower(x) = Text.Lower(y))),

        // q89: Table.Group with comparisonCriteria. Two PQ contracts
        //      verified via the q128-family probes:
        //        - callback receives bare key VALUES (not records) for
        //          single-key grouping
        //        - callback MUST return an ordering -1|0|1 (not logical
        //          — PQ throws "We cannot convert true to Number")
        //      So this uses Value.Compare to return the ordering shape.
        SafeSerialize("q89", () =>
            Table.Group(
                #table({"k","v"}, {{"A",1},{"a",2},{"B",3}}),
                "k",
                {{"total", each List.Sum([v])}},
                GroupKind.Global,
                (a,b) => Value.Compare(Text.Lower(a), Text.Lower(b)))),

        // --- predicate-form arguments (q90-q94) ---

        // q90: List.FirstN take-while.
        SafeSerialize("q90", () =>
            List.FirstN({1,2,3,4,1,2}, each _ < 4)),

        // q91: List.LastN take-while-from-end.
        SafeSerialize("q91", () =>
            List.LastN({1,2,3,4,5,6}, each _ > 3)),

        // q92: List.Skip skip-while.
        SafeSerialize("q92", () =>
            List.Skip({1,2,3,4,3,2,1}, each _ < 4)),

        // q93: Table.FirstN take-while.
        SafeSerialize("q93", () =>
            Table.FirstN(
                #table({"n"}, {{1},{2},{3},{4},{1}}),
                each [n] < 3)),

        // q94: Table.Skip skip-while.
        SafeSerialize("q94", () =>
            Table.Skip(
                #table({"n"}, {{1},{2},{3},{4},{1}}),
                each [n] < 3)),

        // --- quoteStyle / startAtEnd (q95-q102) ---

        // q95: Splitter.SplitTextByDelimiter QuoteStyle.Csv.
        SafeSerialize("q95", () =>
            Splitter.SplitTextByDelimiter(",", QuoteStyle.Csv)(
                "a,""b,c"",d")),

        // q96: Splitter.SplitTextByAnyDelimiter QuoteStyle.Csv.
        SafeSerialize("q96", () =>
            Splitter.SplitTextByAnyDelimiter({",", ";"}, QuoteStyle.Csv)(
                "a,""b;c"",d;e")),

        // q97: Splitter.SplitTextByEachDelimiter forward.
        SafeSerialize("q97", () =>
            Splitter.SplitTextByEachDelimiter({"-", "/"})(
                "abc-def/ghi")),

        // q98: Splitter.SplitTextByEachDelimiter startAtEnd=true.
        SafeSerialize("q98", () =>
            Splitter.SplitTextByEachDelimiter({"-"}, QuoteStyle.None, true)(
                "a-b-c-d")),

        // q99: Splitter.SplitTextByLengths reverse.
        SafeSerialize("q99", () =>
            Splitter.SplitTextByLengths({3, 2}, true)("abcdefg")),

        // q100: Splitter.SplitTextByWhitespace QuoteStyle.Csv.
        SafeSerialize("q100", () =>
            Splitter.SplitTextByWhitespace(QuoteStyle.Csv)(
                "hello ""quoted text"" world")),

        // q101: Combiner.CombineTextByDelimiter QuoteStyle.Csv.
        SafeSerialize("q101", () =>
            Combiner.CombineTextByDelimiter(",", QuoteStyle.Csv)(
                {"a","b,c","d"})),

        // q102: Combiner.CombineTextByDelimiter quotes a field with embedded quote.
        SafeSerialize("q102", () =>
            Combiner.CombineTextByDelimiter(",", QuoteStyle.Csv)(
                {"plain","has ""quote""","newline" & "#(lf)" & "inside"})),

        // --- missingField (q103-q107) ---

        // q103: Record.SelectFields with MissingField.Ignore.
        SafeSerialize("q103", () =>
            Record.SelectFields([a=1, b=2], {"a","missing"}, MissingField.Ignore)),

        // q104: Record.SelectFields with MissingField.UseNull.
        SafeSerialize("q104", () =>
            Record.SelectFields([a=1, b=2], {"a","missing"}, MissingField.UseNull)),

        // q105: Record.RemoveFields with MissingField.Ignore.
        SafeSerialize("q105", () =>
            Record.RemoveFields([a=1, b=2, c=3], {"b","zz"}, MissingField.Ignore)),

        // q106: Record.RenameFields with MissingField.Ignore.
        SafeSerialize("q106", () =>
            Record.RenameFields([a=1, b=2], {{"a","A"},{"zz","ZZ"}},
                MissingField.Ignore)),

        // q107: Record.TransformFields with MissingField.UseNull
        //       — invents the missing field.
        SafeSerialize("q107", () =>
            Record.TransformFields([a=1], {"missing", each if _ = null then 99 else _},
                MissingField.UseNull)),

        // --- occurrence (q108-q112) ---

        // q108: List.PositionOf with Occurrence.Last.
        SafeSerialize("q108", () =>
            List.PositionOf({1,2,3,2,4,2}, 2, Occurrence.Last)),

        // q109: List.PositionOf with Occurrence.All.
        SafeSerialize("q109", () =>
            List.PositionOf({1,2,3,2,4,2}, 2, Occurrence.All)),

        // q110: List.PositionOfAny with Occurrence.All.
        SafeSerialize("q110", () =>
            List.PositionOfAny({1,2,3,2,4,2}, {2,4}, Occurrence.All)),

        // q111: Table.PositionOf with Occurrence.All.
        SafeSerialize("q111", () =>
            Table.PositionOf(
                #table({"k"}, {{"a"},{"b"},{"a"},{"c"}}),
                [k="a"],
                Occurrence.All)),

        // q112: Text.PositionOfAny with Occurrence.All.
        SafeSerialize("q112", () =>
            Text.PositionOfAny("hello world", {"l","o"}, Occurrence.All)),

        // --- one-offs (q113-q120) ---

        // q113: Table.AddRankColumn RankKind.Dense.
        SafeSerialize("q113", () =>
            Table.AddRankColumn(
                #table({"s"}, {{10},{20},{20},{30}}),
                "r",
                "s",
                [RankKind=RankKind.Dense])),

        // q114: Table.AddRankColumn RankKind.Ordinal — every row unique.
        SafeSerialize("q114", () =>
            Table.AddRankColumn(
                #table({"s"}, {{10},{20},{20},{30}}),
                "r",
                "s",
                [RankKind=RankKind.Ordinal])),

        // q115: Table.Group GroupKind.Local — consecutive-run grouping.
        SafeSerialize("q115", () =>
            Table.Group(
                #table({"k","v"}, {{"a",1},{"a",2},{"b",3},{"a",4}}),
                "k",
                {{"total", each List.Sum([v])}},
                GroupKind.Local)),

        // q116: Table.Join composite keys.
        SafeSerialize("q116", () =>
            Table.Join(
                #table({"r","y","s"},
                    {{"EU",2024,10},{"EU",2025,20},{"US",2024,30}}),
                {"r","y"},
                #table({"reg","yr","t"},
                    {{"EU",2024,15},{"EU",2025,25}}),
                {"reg","yr"})),

        // q117: Table.FromValue with options.DefaultColumnName.
        SafeSerialize("q117", () =>
            Table.FromValue(42, [DefaultColumnName="Answer"])),

        // q118: List.Random with seed — reproducible.
        SafeSerialize("q118", () =>
            List.Count(List.Random(5, 42)) = 5),

        // q119: Type.Is type-vs-type subtype check
        //       (the additionalAggregates idiom).
        SafeSerialize("q119", () =>
            Type.Is(type number, type number)),

        // q120: Table.Profile with additionalAggregates — type-driven aggregate.
        SafeSerialize("q120", () =>
            Table.Profile(
                #table({"n","s"}, {{1,"a"},{2,"b"},{3,"c"}}),
                {{"Sum", each Type.Is(_, type number), each List.Sum(_)}})),

        // q121-q127: PQ-canonical Comparer.* idioms. Real PQ accepts
        // built-in comparers as the equationCriteria slot (passed bare,
        // no parens — `Comparer.X` is itself a 2-arg comparer in PQ).
        // Mrsflow now matches this shape and still also accepts our
        // existing custom-lambda extension (verified by q80/q81/q83/etc).

        // q121: List.Distinct with Comparer.OrdinalIgnoreCase.
        SafeSerialize("q121", () =>
            List.Distinct({"a","A","b","B","c"}, Comparer.OrdinalIgnoreCase)),

        // q122: List.Contains with Comparer.OrdinalIgnoreCase.
        SafeSerialize("q122", () =>
            List.Contains({"Hello","World"}, "HELLO", Comparer.OrdinalIgnoreCase)),

        // q123: List.PositionOf with Comparer.OrdinalIgnoreCase.
        SafeSerialize("q123", () =>
            List.PositionOf({"X","Y","z"}, "Z", Occurrence.First,
                Comparer.OrdinalIgnoreCase)),

        // q124: Comparer.Ordinal direct call returns -1 / 0 / 1.
        SafeSerialize("q124", () =>
            Comparer.Ordinal("abc", "abd")),

        // q125: Comparer.OrdinalIgnoreCase direct call: case-fold to equal.
        SafeSerialize("q125", () =>
            Comparer.OrdinalIgnoreCase("ABC", "abc")),

        // q126: Comparer.Equals helper folds the -1/0/1 result to logical.
        SafeSerialize("q126", () =>
            Comparer.Equals(Comparer.OrdinalIgnoreCase, "ABC", "abc")),

        // q127: Comparer.FromCulture(_, true) is case-insensitive.
        SafeSerialize("q127", () =>
            List.Distinct({"a","A","b"}, Comparer.FromCulture("en-US", true))),

        // q128-q149: probes for the "PQ refuses lambda" family. Each
        // function tested with 3 variants:
        //   a) lambda-logical (mrsflow accepts; PQ throws — dump try record)
        //   b) lambda-ordering — does PQ accept the Table.Group shape?
        //   c) PQ-canonical Comparer.* — works in both
        // If (b) succeeds in PQ, mrsflow needs the same shape contract.

        // --- List.Distinct (q128-q130) ---

        // Probe shape: drop Error.Detail (which may contain non-
        // serializable types) so Json.FromValue doesn't choke.
        SafeSerialize("q128", () =>
            let
                r = try List.Distinct({"a","A","b","B","c"},
                    (x,y) => Text.Lower(x) = Text.Lower(y))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q129", () =>
            let
                r = try List.Distinct({"a","A","b","B","c"},
                    (x,y) => Value.Compare(Text.Lower(x), Text.Lower(y)))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q130", () =>
            List.Distinct({"a","A","b","B","c"}, Comparer.OrdinalIgnoreCase)),

        // --- List.Intersect (q131-q133) ---

        SafeSerialize("q131", () =>
            let
                r = try List.Intersect({{"A","B","C"}, {"a","b"}},
                    (x,y) => Text.Lower(x) = Text.Lower(y))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q132", () =>
            let
                r = try List.Intersect({{"A","B","C"}, {"a","b"}},
                    (x,y) => Value.Compare(Text.Lower(x), Text.Lower(y)))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q133", () =>
            List.Intersect({{"A","B","C"}, {"a","b"}},
                Comparer.OrdinalIgnoreCase)),

        // --- List.Difference (q134-q136) ---

        SafeSerialize("q134", () =>
            let
                r = try List.Difference({"A","B","C"}, {"a","c"},
                    (x,y) => Text.Lower(x) = Text.Lower(y))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q135", () =>
            let
                r = try List.Difference({"A","B","C"}, {"a","c"},
                    (x,y) => Value.Compare(Text.Lower(x), Text.Lower(y)))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q136", () =>
            List.Difference({"A","B","C"}, {"a","c"},
                Comparer.OrdinalIgnoreCase)),

        // --- Table.Distinct (q137-q140) ---

        SafeSerialize("q137", () =>
            let
                r = try Table.Distinct(
                    #table({"k","v"}, {{"a",1},{"A",2},{"b",3}}),
                    (r1,r2) => Text.Lower(r1[k]) = Text.Lower(r2[k]))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q138", () =>
            let
                r = try Table.Distinct(
                    #table({"k","v"}, {{"a",1},{"A",2},{"b",3}}),
                    (r1,r2) => Value.Compare(Text.Lower(r1[k]), Text.Lower(r2[k])))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // Single column-name as criteria (per docs, allowed shape).
        SafeSerialize("q139", () =>
            Table.Distinct(
                #table({"k","v"}, {{"a",1},{"A",2},{"b",3}}),
                "k")),

        // List of {column, comparer} per docs.
        SafeSerialize("q140", () =>
            let
                r = try Table.Distinct(
                    #table({"k","v"}, {{"a",1},{"A",2},{"b",3}}),
                    {"k", Comparer.OrdinalIgnoreCase})
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // --- Value.Equals (q141-q144) ---

        SafeSerialize("q141", () =>
            let
                r = try Value.Equals("Hello", "HELLO",
                    (a,b) => Text.Lower(a) = Text.Lower(b))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q142", () =>
            let
                r = try Value.Equals("Hello", "HELLO",
                    (a,b) => Value.Compare(Text.Lower(a), Text.Lower(b)))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q143", () =>
            Value.Equals("Hello", "HELLO", Comparer.OrdinalIgnoreCase)),

        SafeSerialize("q144", () =>
            Value.Equals("Hello", "HELLO")),

        // --- Table.Sort comparisonCriteria (q145-q148) ---

        SafeSerialize("q145", () =>
            let
                r = try Table.Sort(
                    #table({"k"}, {{"b"},{"A"},{"a"},{"C"}}),
                    (r1,r2) => Value.Compare(Text.Lower(r1[k]), Text.Lower(r2[k])))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q146", () =>
            Table.Sort(
                #table({"k"}, {{"b"},{"A"},{"a"},{"C"}}),
                "k")),

        SafeSerialize("q147", () =>
            Table.Sort(
                #table({"k"}, {{"b"},{"A"},{"a"},{"C"}}),
                {"k", Order.Descending})),

        SafeSerialize("q148", () =>
            let
                r = try Table.Sort(
                    #table({"k"}, {{"b"},{"A"},{"a"},{"C"}}),
                    {{"k", Order.Ascending, Comparer.OrdinalIgnoreCase}})
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // --- List.Sort (q149) ---

        SafeSerialize("q149", () =>
            let
                r = try List.Sort({3,1,4,1,5,9,2,6},
                    (a,b) => Value.Compare(b, a))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q150-q153: q139 follow-up. q139's data has all-distinct keys
        // by accident, so "no-op" and "dedup-by-k" look the same. These
        // probes use data with real duplicates so we can tell which
        // PQ semantics is actually in play.

        // q150: ACTUAL duplicates in column k. If PQ dedups by k,
        //       result has 2 rows ({a,1} and {b,3} — first-seen).
        //       If PQ is a true no-op, result has 3 rows.
        SafeSerialize("q150", () =>
            Table.Distinct(
                #table({"k","v"}, {{"a",1},{"a",2},{"b",3}}),
                "k")),

        // q151: Different VALUE-column for matching k. Tells us whether
        //       PQ keeps first-seen, last-seen, or something else.
        SafeSerialize("q151", () =>
            Table.Distinct(
                #table({"k","v"}, {{"a","first"},{"a","second"},{"b","third"}}),
                "k")),

        // q152: list-of-columns form per docs. Dedup by ALL listed cols?
        SafeSerialize("q152", () =>
            Table.Distinct(
                #table({"k","v","w"},
                    {{"a",1,"x"},{"a",1,"y"},{"a",2,"z"},{"b",1,"w"}}),
                {"k","v"})),

        // q153: list-with-comparer multi-column. Speculation: like q140
        //       but applies the comparer per-column?
        SafeSerialize("q153", () =>
            let
                r = try Table.Distinct(
                    #table({"k","v"}, {{"a",1},{"A",2},{"b",3}}),
                    {{"k", Comparer.OrdinalIgnoreCase}})
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q154-q175: focused-coverage probes across uncharted territory.

        // --- Number.ToText format strings (q154-q159) ---

        SafeSerialize("q154", () => Number.ToText(3.14159, "F2")),
        SafeSerialize("q155", () => Number.ToText(1234567, "N0")),
        SafeSerialize("q156", () => Number.ToText(0.123, "P1")),
        SafeSerialize("q157", () => Number.ToText(1234.5, "E2")),
        SafeSerialize("q158", () => Number.ToText(42, "D5")),
        SafeSerialize("q159", () =>
            let r = try Number.ToText(99.5, "C") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // --- DateTime/Time.ToText format strings (q160-q162) ---

        SafeSerialize("q160", () =>
            DateTime.ToText(#datetime(2026,6,15,14,30,45), "yyyy-MM-ddTHH:mm:ss")),

        SafeSerialize("q161", () =>
            let r = try DateTime.ToText(#datetime(2026,6,15,14,30,45), "g") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q162", () =>
            let r = try Time.ToText(#time(14,30,0), "hh:mm tt") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // --- List.Accumulate (q163-q165) ---

        SafeSerialize("q163", () =>
            List.Accumulate({1,2,3,4,5}, 0, (state, current) => state + current)),

        SafeSerialize("q164", () =>
            List.Accumulate({"a","b","c"}, {}, (state, current) => state & {current})),

        SafeSerialize("q165", () =>
            List.Accumulate({}, 42, (state, current) => state + current)),

        // --- Table.ReplaceValue (q166-q168) ---

        SafeSerialize("q166", () =>
            Table.ReplaceValue(
                #table({"v"}, {{1},{2},{1}}),
                1,
                99,
                Replacer.ReplaceValue,
                {"v"})),

        SafeSerialize("q167", () =>
            Table.ReplaceValue(
                #table({"s"}, {{"foo bar"},{"baz foo"}}),
                "foo",
                "FOO",
                Replacer.ReplaceText,
                {"s"})),

        SafeSerialize("q168", () =>
            Table.ReplaceValue(
                #table({"v"}, {{1},{2}}),
                99,
                "NEVER",
                Replacer.ReplaceValue,
                {"v"})),

        // --- Operator type coercion (q169-q173) ---

        SafeSerialize("q169", () =>
            let r = try 1 + "2" in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q170", () =>
            let r = try null + 1 in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q171", () =>
            #date(2026,12,31) - #date(2026,1,1)),

        SafeSerialize("q172", () => "hello" & " " & "world"),

        SafeSerialize("q173", () =>
            let r = try "n=" & 42 in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // --- Exotic splitters (q174-q175) ---

        SafeSerialize("q174", () =>
            Splitter.SplitTextByCharacterTransition(
                {"a".."z"}, {"0".."9"})("abc123def456")),

        SafeSerialize("q175", () =>
            Splitter.SplitTextByRepeatedLengths(2)("abcdefgh")),

        // q176-q180: Number.ToText format strings v2 — extended coverage.

        SafeSerialize("q176", () =>
            let r = try Number.ToText(3.7, "F0") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q177", () =>
            let r = try Number.ToText(1234567.891, "N2") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q178", () =>
            let r = try Number.ToText(1234.5, "#,##0.00") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q179", () =>
            let r = try Number.ToText(0.456, "0.00%") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q180", () =>
            let r = try Number.ToText(-1234.567, "F2") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q181-q185: DateTime/Date/Time/DateTimeZone.ToText format codes.

        SafeSerialize("q181", () =>
            let r = try Date.ToText(#date(2026,6,15), "d") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q182", () =>
            let r = try Date.ToText(#date(2026,6,15), "yyyy") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q183", () =>
            let r = try DateTime.ToText(#datetime(2026,6,15,14,30,45), "f") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q184", () =>
            let r = try DateTime.ToText(#datetime(2026,6,15,14,30,45), "O") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q185", () =>
            let r = try DateTimeZone.ToText(
                #datetimezone(2026,6,15,14,30,45,1,0), "K") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q186-q190: Replacer.* + Text.Replace.

        SafeSerialize("q186", () =>
            let r = try Replacer.ReplaceValue(5, 5, 99) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q187", () =>
            let r = try Replacer.ReplaceText("hello world", "world", "PQ") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q188", () =>
            let r = try Text.Replace("abc", "", "X") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q189", () =>
            Text.Replace("aaaa", "aa", "b")),

        SafeSerialize("q190", () =>
            let r = try Table.ReplaceValue(
                #table({"v"}, {{1},{2},{1}}),
                1, 99, Replacer.ReplaceValue, {"v"}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q191-q195: Text.Format placeholder substitution.

        SafeSerialize("q191", () =>
            let r = try Text.Format("#{0} = #{1}", {"x", 42}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q192", () =>
            let r = try Text.Format("Hello, #{name}!", [name="world"]) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q193", () =>
            let r = try Text.Format("#{0} and #{1}", {"only_one"}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q194", () =>
            let r = try Text.Format("value=#{0}", {3.14}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q195", () =>
            let r = try Text.Format("a##b#{0}c", {"x"}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q196-q200: List.Accumulate edge cases.

        SafeSerialize("q196", () =>
            let r = try List.Accumulate({1,2,3}, {}, (s,c) => s & {c*2}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q197", () =>
            List.Accumulate({"a","b","c"}, "", (s,c) => s & "[" & c & "]")),

        SafeSerialize("q198", () =>
            List.Accumulate({1,2,3}, {{}}, (s,c) => s & {{c, c*c}})),

        SafeSerialize("q199", () =>
            let r = try List.Accumulate({1,2,3}, 0,
                (s,c) => if c = 2 then error "boom" else s + c) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q200", () =>
            List.Accumulate({1..100}, 0, (s,c) => s + c)),

        // q201-q205: try ... otherwise.

        SafeSerialize("q201", () =>
            try 1/0 otherwise -1),

        SafeSerialize("q202", () =>
            try 1+1 otherwise -1),

        SafeSerialize("q203", () =>
            try (1 + "x") otherwise "fallback"),

        SafeSerialize("q204", () =>
            try (error "kaboom") otherwise "ok"),

        SafeSerialize("q205", () =>
            try (try 1/0 otherwise error "rethrow") otherwise "caught"),

        // q206-q210: error "msg" and error-record construction.

        SafeSerialize("q206", () =>
            let r = try error "boom" in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q207", () =>
            let r = try error [Reason="Custom.Reason", Message="msg-here", Detail="details"] in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q208", () =>
            let r = try error 42 in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q209", () =>
            let r = try error Error.Record("X.Y", "the message", "the detail") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q210", () =>
            let
                inner = try error "first",
                r = try error inner[Error]
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q211-q215: Operator coercion deep-dive.

        SafeSerialize("q211", () =>
            let r = try null * 5 in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q212", () =>
            let r = try null - null in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q213", () =>
            let r = try 1 < "1" in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q214", () =>
            let r = try 1 = "1" in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q215", () =>
            let r = try null & "x" in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q216-q220: Duration serialization.

        SafeSerialize("q216", () =>
            let r = try Duration.ToText(#duration(1,2,3,4)) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q217", () =>
            let r = try Duration.FromText("1.02:03:04") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q218", () =>
            let r = try Duration.From(3600) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q219", () =>
            Duration.TotalSeconds(#duration(0,1,30,0))),

        SafeSerialize("q220", () =>
            #duration(1,2,3,4)),

        // q221-q225: Date arithmetic edge cases.

        SafeSerialize("q221", () => Date.AddMonths(#date(2026,1,31), 1)),
        SafeSerialize("q222", () => Date.AddYears(#date(2024,2,29), 1)),
        SafeSerialize("q223", () => Date.AddYears(#date(2024,2,29), 4)),
        SafeSerialize("q224", () => Date.AddDays(#date(2026,1,1), -1)),
        SafeSerialize("q225", () => Date.AddQuarters(#date(2026,1,15), 3)),

        // q226-q230: Table.AddColumn 4th arg (column type ascription).

        SafeSerialize("q226", () =>
            Table.AddColumn(#table({"A"}, {{10}}), "B", each [A] * 2, Int64.Type)),

        SafeSerialize("q227", () =>
            Table.AddColumn(#table({"A"}, {{10}}), "B", each [A] * 2.5, type number)),

        SafeSerialize("q228", () =>
            Table.AddColumn(#table({"A"}, {{1}}), "label", each "row-" & Text.From([A]), type text)),

        SafeSerialize("q229", () =>
            Table.AddColumn(#table({"d"}, {{#date(2026,1,1)}}),
                "next", each Date.AddDays([d], 1), type date)),

        SafeSerialize("q230", () =>
            Table.AddColumn(#table({"A"}, {{10}}), "B", each [A] * 2)),

        // q231-q235: Record.AddField delayed flag.

        SafeSerialize("q231", () =>
            Record.AddField([a=1], "b", 99)),

        SafeSerialize("q232", () =>
            // Don't return the raw delayed-record — mrsflow's Json.FromValue
            // chokes on function-valued fields, halting catalog capture.
            // Surface just the field names + whether construction succeeded.
            let r = try Record.AddField([a=1], "b", () => 99, true) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else Record.FieldNames(r[Value])),

        SafeSerialize("q233", () =>
            // PQ forces the delayed field on access; mrsflow returns the
            // closure itself. We invoke it explicitly so the case can
            // serialize regardless.
            let v = Record.AddField([a=1], "b", () => 99, true)[b] in
                if Value.Is(v, type function) then v() else v),

        SafeSerialize("q234", () =>
            // Same shape as q232 — return field-names list, not the raw record.
            let r = try Record.AddField([a=1], "bad", () => error "x", true) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else Record.FieldNames(r[Value])),

        SafeSerialize("q235", () =>
            // Force the delayed field; in mrsflow that means invoking the
            // closure manually since [bad] doesn't auto-force.
            let r = try
                let v = Record.AddField([a=1], "bad", () => error "x", true)[bad] in
                    if Value.Is(v, type function) then v() else v
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q236-q240: Csv.Document encoding edge cases.

        SafeSerialize("q236", () =>
            Csv.Document(Text.ToBinary("a,b#(lf)1,2"))),

        SafeSerialize("q237", () =>
            let r = try Csv.Document(
                Text.ToBinary("a,b#(lf)1,2"),
                [Encoding=65001]) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q238", () =>
            let r = try Csv.Document(
                Text.ToBinary("a,b#(lf)1,2"),
                [Encoding=1252]) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q239", () =>
            let r = try Csv.Document(
                Binary.Combine({#binary({0xEF,0xBB,0xBF}), Text.ToBinary("a,b#(lf)1,2")})) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q240", () =>
            Csv.Document(Text.ToBinary(""))),

        // q241-q245: #shared introspection. The whole record contains
        // function values that Json.FromValue can't serialize directly,
        // so probe only counts and field-presence.

        SafeSerialize("q241", () =>
            let r = try Record.FieldCount(#shared) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q242", () =>
            let r = try Record.HasFields(#shared, "Number.From") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q243", () =>
            let r = try Record.HasFields(#shared, "Number.NonexistentXYZ") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q244", () =>
            let r = try Value.Is(#shared[Number.From], type function) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q245", () =>
            let r = try #shared[Number.From]("42") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q246-q250: Splitter.SplitTextByCharacterTransition char-set
        // notations. q174 showed the {"a".."z"} range syntax fails in
        // mrsflow — these probes try alternatives.

        SafeSerialize("q246", () =>
            let r = try Splitter.SplitTextByCharacterTransition(
                {"a","b","c"}, {"0","1","2","3","4","5","6","7","8","9"})("abc123") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q247", () =>
            let r = try Splitter.SplitTextByCharacterTransition(
                {"a".."z"}, {"0".."9"})("hello123world456") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q248", () =>
            let r = try Splitter.SplitTextByCharacterTransition(
                {"0".."9"}, {"a".."z"})("123hello456world") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q249", () =>
            let r = try Splitter.SplitTextByCharacterTransition(
                {"a","b","c","d","e","f","g","h","i","j","k","l","m","n","o","p","q","r","s","t","u","v","w","x","y","z"},
                {"0","1","2","3","4","5","6","7","8","9"})("hello123world456") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q250", () =>
            let r = try Splitter.SplitTextByCharacterTransition({}, {})("abc") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q251-q255: Json.Document edge cases.

        SafeSerialize("q251", () =>
            let r = try Json.Document("[1.5e3]") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q252", () =>
            let r = try Json.Document("[""é""]") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q253", () =>
            let r = try Json.Document("[[[1,2],[3,4]],[[5,6]]]") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q254", () =>
            let r = try Json.Document("{""k"":[]}") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q255", () =>
            let r = try Json.Document("9007199254740993") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q256-q260: List.Sort / Table.Sort stability on equal keys.

        SafeSerialize("q256", () =>
            List.Sort(
                {[k=2,i=1], [k=1,i=2], [k=2,i=3], [k=1,i=4]},
                each _[k])),

        SafeSerialize("q257", () =>
            List.Sort(
                {[k=2,i=3], [k=1,i=4], [k=2,i=1], [k=1,i=2]},
                each _[k])),

        SafeSerialize("q258", () =>
            Table.Sort(
                #table({"k","i"}, {{2,1},{1,2},{2,3},{1,4}}),
                "k")),

        SafeSerialize("q259", () =>
            List.Sort({2,1,2,1,2})),

        SafeSerialize("q260", () =>
            List.Sort({"B","a","A","b"}, Comparer.OrdinalIgnoreCase)),

        // q261-q265: Table.Pivot / Table.Unpivot.

        SafeSerialize("q261", () =>
            let r = try Table.Unpivot(
                #table({"id","jan","feb","mar"}, {{"a",1,2,3}}),
                {"jan","feb","mar"}, "month", "value") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q262", () =>
            let r = try Table.UnpivotOtherColumns(
                #table({"id","jan","feb"}, {{"a",1,2}, {"b",3,4}}),
                {"id"}, "month", "value") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q263", () =>
            let r = try Table.Pivot(
                #table({"id","month","value"},
                    {{"a","jan",1},{"a","feb",2},{"a","mar",3}}),
                {"jan","feb","mar"}, "month", "value") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q264", () =>
            let r = try Table.Pivot(
                #table({"id","month","value"},
                    {{"a","jan",1},{"a","jan",10},{"a","feb",2}}),
                {"jan","feb"}, "month", "value", List.Sum) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q265", () =>
            let
                orig = #table({"id","jan","feb"}, {{"a",1,2}, {"b",3,4}}),
                unp = Table.UnpivotOtherColumns(orig, {"id"}, "month", "value"),
                r = try Table.Pivot(unp, {"jan","feb"}, "month", "value")
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q266-q270: Json.FromValue handling of function-typed values.

        SafeSerialize("q266", () =>
            let r = try Text.FromBinary(Json.FromValue([a=1, b=2]), TextEncoding.Utf8) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q267", () =>
            let r = try Text.FromBinary(
                Json.FromValue([a=1, f=(x) => x+1]),
                TextEncoding.Utf8) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q268", () =>
            let r = try Text.FromBinary(
                Json.FromValue({1, (x) => x*2, 3}),
                TextEncoding.Utf8) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q269", () =>
            let r = try Text.FromBinary(
                Json.FromValue((x) => x),
                TextEncoding.Utf8) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q270", () =>
            let r = try Text.FromBinary(
                Json.FromValue([Name="x", Compute=(n) => n+1]),
                TextEncoding.Utf8) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q271-q275: Text.From scalar coercion.

        SafeSerialize("q271", () => Text.From(null)),

        SafeSerialize("q272", () => Text.From(true)),

        SafeSerialize("q273", () => Text.From(#date(2026,6,15))),

        SafeSerialize("q274", () =>
            let r = try Text.From({1,2,3}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q275", () => Text.From(123456789012345)),

        // q276-q280: Number arithmetic — Decimal vs Number.

        SafeSerialize("q276", () => 0.1 + 0.2),

        SafeSerialize("q277", () =>
            let r = try Decimal.From(0.1) + Decimal.From(0.2) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q278", () =>
            let r = try Number.IsNaN(Number.NaN) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q279", () =>
            let r = try Number.IsOdd(7) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q280", () =>
            let r = try Decimal.From("0.1") * 3 in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q281-q285: List.Generate loop semantics.

        SafeSerialize("q281", () =>
            List.Generate(() => 1, each _ <= 5, each _ + 1)),

        SafeSerialize("q282", () =>
            List.Generate(() => 0, each _ < 0, each _ + 1)),

        SafeSerialize("q283", () =>
            List.Generate(() => 1, each _ <= 3, each _ + 1, each _ * 10)),

        SafeSerialize("q284", () =>
            List.Generate(
                () => [i=0, total=0],
                each [i] < 4,
                each [i=[i]+1, total=[total]+[i]+1],
                each [total])),

        SafeSerialize("q285", () =>
            List.Generate(() => 5, each _ = 5, each _ + 1)),

        // q286-q290: Record.Combine collisions + Record.Field.

        SafeSerialize("q286", () =>
            Record.Combine({[a=1], [b=2]})),

        SafeSerialize("q287", () =>
            let r = try Record.Combine({[a=1], [a=2]}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q288", () =>
            let r = try Record.Combine({[a=1, b=2], [b=20, c=3], [c=30]}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q289", () =>
            Record.Combine({})),

        SafeSerialize("q290", () =>
            let r = try Record.Field(Record.Combine({[a=1], [a=2]}), "a") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q291-q295: Table.Schema + Table.ColumnsOfType.

        SafeSerialize("q291", () =>
            let r = try Table.Schema(
                Table.TransformColumnTypes(
                    #table({"n","s","b"}, {{1,"x",true}}),
                    {{"n", Int64.Type}, {"s", type text}, {"b", type logical}}))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q292", () =>
            let r = try Table.RowCount(Table.Schema(
                Table.TransformColumnTypes(
                    #table({"n","s","b"}, {{1,"x",true}}),
                    {{"n", Int64.Type}, {"s", type text}, {"b", type logical}})))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q293", () =>
            let r = try Table.ColumnsOfType(
                Table.TransformColumnTypes(
                    #table({"n","s"}, {{1,"x"}}),
                    {{"n", Int64.Type}, {"s", type text}}),
                {type number})
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q294", () =>
            let r = try Table.ColumnsOfType(
                Table.TransformColumnTypes(
                    #table({"n","s","b"}, {{1,"x",true}}),
                    {{"n", Int64.Type}, {"s", type text}, {"b", type logical}}),
                {type number, type text})
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q295", () =>
            let r = try Table.ColumnNames(Table.Schema(#table({"a","b"}, {{1,2}}))) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q296-q300: Lines.* — newline conventions + roundtrips.

        SafeSerialize("q296", () =>
            Lines.FromText("a#(lf)b#(lf)c")),

        SafeSerialize("q297", () =>
            Lines.FromText("a#(cr)#(lf)b#(cr)#(lf)c")),

        SafeSerialize("q298", () =>
            Lines.FromText("a#(lf)b#(lf)")),

        SafeSerialize("q299", () =>
            Lines.FromText("")),

        SafeSerialize("q300", () =>
            Lines.ToText(Lines.FromText("a#(lf)b#(lf)c"))),

        // q301-q305: Text.Split/Combine roundtrips.

        SafeSerialize("q301", () =>
            Text.Combine(Text.Split("a,b,c", ","), ",")),

        SafeSerialize("q302", () =>
            Text.Combine(Text.Split("a,b,", ","), ",")),

        SafeSerialize("q303", () =>
            Text.Split("a,,b", ",")),

        SafeSerialize("q304", () =>
            Text.SplitAny("a;b,c|d", ",;|")),

        SafeSerialize("q305", () =>
            Text.Combine(Text.Split("", ","), ",")),

        // q306-q310: List.Combine/Zip/RemoveItems edge cases.

        SafeSerialize("q306", () =>
            List.Combine({{1,2},{3},{}})),

        SafeSerialize("q307", () =>
            List.Zip({{1,2,3},{"a","b"}})),

        SafeSerialize("q308", () =>
            List.Zip({})),

        SafeSerialize("q309", () =>
            List.RemoveItems({1,2,3,2,4}, {2})),

        SafeSerialize("q310", () =>
            List.Combine({})),

        // q311-q315: Date.Day vs DayOfWeek/DayOfYear/DaysInMonth/DayOfWeekName.

        SafeSerialize("q311", () => Date.Day(#date(2026,3,15))),
        SafeSerialize("q312", () => Date.DayOfWeek(#date(2026,3,15))),
        SafeSerialize("q313", () => Date.DayOfWeekName(#date(2026,3,15))),
        SafeSerialize("q314", () => Date.DayOfYear(#date(2026,3,15))),
        SafeSerialize("q315", () => Date.DaysInMonth(#date(2026,3,15))),

        // q316-q320: Number.IsOdd/IsEven/Mod/IntegerDivide sign semantics.

        SafeSerialize("q316", () => Number.IsOdd(-7)),
        SafeSerialize("q317", () => Number.IsEven(-7)),
        SafeSerialize("q318", () => Number.Mod(-10, 3)),
        SafeSerialize("q319", () => Number.IntegerDivide(-10, 3)),
        SafeSerialize("q320", () => Number.Mod(10, -3)),

        // q321-q325: List.Min/Max with mixed/edge types.

        SafeSerialize("q321", () => List.Max({1, null, 5})),

        SafeSerialize("q322", () => List.Min({null, null, null})),

        SafeSerialize("q323", () => List.Max({"a", "b", "c"})),

        SafeSerialize("q324", () =>
            let r = try List.Max({1, "x"}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q325", () => List.Max({})),

        // q326-q330: List.Generate early-termination patterns.

        SafeSerialize("q326", () =>
            List.Generate(
                () => [a=0, b=1],
                each [a] <= 100,
                each [a=[b], b=[a]+[b]],
                each [a])),

        SafeSerialize("q327", () =>
            List.Generate(
                () => [n=1, done=false],
                each not [done],
                each [n=[n]+1, done=([n]+1) >= 5],
                each [n])),

        SafeSerialize("q328", () =>
            List.Generate(() => 42, each _ = 42, each _ + 1)),

        SafeSerialize("q329", () =>
            List.Generate(() => 42, each false, each _ + 1)),

        SafeSerialize("q330", () =>
            List.Sum(List.Generate(() => 1, each _ <= 50, each _ + 1))),

        // q331-q335: Record.*Fields with empty list args.

        SafeSerialize("q331", () =>
            Record.SelectFields([a=1, b=2], {})),

        SafeSerialize("q332", () =>
            Record.SelectFields([], {})),

        SafeSerialize("q333", () =>
            Record.RemoveFields([a=1, b=2], {})),

        SafeSerialize("q334", () =>
            Record.RenameFields([a=1, b=2], {})),

        SafeSerialize("q335", () =>
            Record.ReorderFields([a=1, b=2], {})),

        // q336-q340: Table.Fuzzy* family (likely unsupported in mrsflow).

        SafeSerialize("q336", () =>
            let r = try Table.FuzzyJoin(
                #table({"k"}, {{"apple"},{"banana"}}),
                "k",
                #table({"kr"}, {{"appel"},{"bananna"}}),
                "kr")
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q337", () =>
            let r = try Table.FuzzyNestedJoin(
                #table({"k"}, {{"apple"}}),
                {"k"},
                #table({"kr"}, {{"appel"}}),
                {"kr"},
                "right")
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q338", () =>
            let r = try Table.FuzzyGroup(
                #table({"k"}, {{"apple"},{"appel"},{"banana"}}),
                "k",
                {{"items", each _}})
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q339", () =>
            let r = try Table.FuzzyJoin(
                #table({"k"}, {{"apple"},{"banana"}}),
                "k",
                #table({"kr"}, {{"appel"}}),
                "kr",
                JoinKind.Inner,
                [SimilarityThreshold=0.8])
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q340", () =>
            let r = try Table.FuzzyJoin(
                #table({"k"}, {{"apple"}}),
                "k",
                #table({"kr"}, {{"apple"}}),
                "kr",
                JoinKind.Inner,
                [SimilarityThreshold=1.0])
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q341", () =>
            let r = try List.Sum(List.Buffer({1, 2, 3, 4, 5})) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q342", () =>
            let r = try List.Buffer({}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q343", () =>
            let r = try Table.RowCount(Table.Buffer(#table({"a"}, {{1}, {2}, {3}}))) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q344", () =>
            let r = try
                let
                    buffered = List.Buffer({"x", "y", "z"}),
                    first = buffered{0},
                    last = buffered{2}
                in
                    first & "-" & last
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q345", () =>
            let r = try Table.ColumnNames(Table.Buffer(#table({"col1", "col2"}, {{1, "a"}, {2, "b"}}))) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q346", () =>
            let r = try Binary.Length(Binary.FromText("SGVsbG8=", BinaryEncoding.Base64)) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q347", () =>
            let r = try Binary.ToText(Binary.FromText("48656c6c6f", BinaryEncoding.Hex), BinaryEncoding.Base64) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q348", () =>
            let r = try Binary.ToText(Binary.Range(Binary.FromText("48656c6c6f20576f726c64", BinaryEncoding.Hex), 6, 5), BinaryEncoding.Hex) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q349", () =>
            let r = try Binary.Length(Binary.FromText("", BinaryEncoding.Base64)) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q350", () =>
            let r = try Binary.ToText(Text.ToBinary("Hello", TextEncoding.Utf8), BinaryEncoding.Hex) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q351", () =>
            let r = try Binary.ToText(Binary.FromList({0, 15, 16, 255}), BinaryEncoding.Hex) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q352", () =>
            let r = try Binary.ToText(Binary.FromList({0, 0, 0, 0}), BinaryEncoding.Hex) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q353", () =>
            let r = try Binary.ToText(Binary.FromList({}), BinaryEncoding.Hex) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q354", () =>
            let r = try
                let
                    orig = Binary.FromList({170, 187, 204, 221}),
                    hex = Binary.ToText(orig, BinaryEncoding.Hex),
                    roundtrip = Binary.FromText(hex, BinaryEncoding.Hex),
                    equal = Binary.ToText(roundtrip, BinaryEncoding.Base64) = Binary.ToText(orig, BinaryEncoding.Base64)
                in
                    equal
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q355", () =>
            let r = try Binary.ToText(Binary.FromList({1}), BinaryEncoding.Hex) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q356", () =>
            let r = try {Number.Round(0.5), Number.Round(1.5), Number.Round(2.5), Number.Round(-0.5), Number.Round(-1.5)} in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q357", () =>
            let r = try {Number.Round(3.14159, 2), Number.Round(3.14159, 3), Number.Round(3.14159, 0), Number.Round(123.456, -1)} in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q358", () =>
            let r = try {Number.RoundUp(2.1), Number.RoundUp(-2.1), Number.RoundUp(2.9), Number.RoundUp(-2.9), Number.RoundUp(0)} in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q359", () =>
            let r = try {Number.RoundDown(2.9), Number.RoundDown(-2.9), Number.RoundDown(2.1), Number.RoundDown(-2.1), Number.RoundDown(0)} in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q360", () =>
            let r = try {
                    Number.Round(0.5, 0, RoundingMode.AwayFromZero),
                    Number.Round(0.5, 0, RoundingMode.ToEven),
                    Number.Round(1.5, 0, RoundingMode.ToEven),
                    Number.Round(-0.5, 0, RoundingMode.AwayFromZero),
                    Number.Round(2.5, 0, RoundingMode.ToEven)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q361", () =>
            let r = try {Text.PadStart("42", 5), Text.PadStart("42", 5, "0"), Text.PadStart("hi", 2), Text.PadStart("", 3, "*")} in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q362", () =>
            let r = try {Text.PadEnd("42", 5), Text.PadEnd("42", 5, "."), Text.PadEnd("hi", 2), Text.PadEnd("", 3, "x")} in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q363", () =>
            let r = try {Text.Repeat("ab", 3), Text.Repeat("x", 0), Text.Repeat("", 5), Text.Repeat("-", 10)} in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q364", () =>
            let r = try Text.PadStart("hi", 6, "ab") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q365", () =>
            let r = try Text.Repeat("ab", -1) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q366", () =>
            let r = try {
                    List.PositionOf({"a", "b", "c", "b", "a"}, "b"),
                    List.PositionOf({"a", "b", "c", "b", "a"}, "z"),
                    List.PositionOf({}, "x"),
                    List.PositionOf({1, 2, 3}, 2)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q367", () =>
            let r = try List.PositionOf({"a", "b", "c", "b", "a"}, "b", Occurrence.All) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q368", () =>
            let r = try {
                    List.PositionOfAny({"a", "b", "c"}, {"b", "z"}),
                    List.PositionOfAny({"a", "b", "c"}, {"z", "y"}),
                    List.PositionOfAny({"a", "b", "c"}, {})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q369", () =>
            let r = try {
                    List.Contains({"a", "b", "c"}, "b"),
                    List.Contains({"a", "b", "c"}, "z"),
                    List.Contains({}, "x"),
                    List.Contains({1, 2, null}, null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q370", () =>
            let r = try List.PositionOf({"A", "b", "C"}, "a", Occurrence.First, Comparer.OrdinalIgnoreCase) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q371", () =>
            let r = try Splitter.SplitByNothing()("hello") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q372", () =>
            let r = try Splitter.SplitTextByDelimiter(",")("a,b,c,d") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q373", () =>
            let r = try Splitter.SplitTextByDelimiter(",", QuoteStyle.Csv)("a,""b,c"",d") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q374", () =>
            let r = try Splitter.SplitTextByEachDelimiter({",", ";", "|"})("a,b;c|d,e") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q375", () =>
            let r = try Splitter.SplitTextByLengths({2, 3, 1})("abcdefg") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q376", () =>
            let r = try Combiner.CombineTextByDelimiter(",")({"a", "b", "c", "d"}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q377", () =>
            let r = try Combiner.CombineTextByDelimiter(",", QuoteStyle.Csv)({"a", "b,c", "d""e"}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q378", () =>
            let r = try Combiner.CombineTextByEachDelimiter({",", ";", "|"})({"a", "b", "c", "d"}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q379", () =>
            let r = try Combiner.CombineTextByLengths({2, 3, 1})({"ab", "cde", "f"}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q380", () =>
            let r = try Combiner.CombineTextByPositions({0, 5, 10})({"abc", "defg", "hi"}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q381", () =>
            let r = try {Comparer.Ordinal("a", "b"), Comparer.Ordinal("b", "a"), Comparer.Ordinal("a", "a"), Comparer.Ordinal("A", "a")} in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q382", () =>
            let r = try {
                    Comparer.OrdinalIgnoreCase("a", "A"),
                    Comparer.OrdinalIgnoreCase("a", "B"),
                    Comparer.OrdinalIgnoreCase("B", "a"),
                    Comparer.OrdinalIgnoreCase("Hello", "HELLO")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q383", () =>
            let r = try List.Sort({"banana", "Apple", "cherry", "apple"}, Comparer.OrdinalIgnoreCase) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q384", () =>
            let r = try
                let c = Comparer.FromCulture("en-US", true) in
                    {c("a", "A"), c("a", "B"), c("z", "a")}
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q385", () =>
            let r = try {Comparer.Ordinal(1, 2), Comparer.Ordinal(2, 2), Comparer.Ordinal(3, 1), Comparer.Ordinal(null, 1)} in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q386", () =>
            let r = try {
                    Value.Equals(1, 1),
                    Value.Equals(1, 1.0),
                    Value.Equals("a", "a"),
                    Value.Equals("a", "A"),
                    Value.Equals(null, null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q387", () =>
            let r = try {
                    Value.Equals(1, "1"),
                    Value.Equals(true, 1),
                    Value.Equals({1, 2}, {1, 2}),
                    Value.Equals({1, 2}, {2, 1}),
                    Value.Equals([a=1, b=2], [b=2, a=1])
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q388", () =>
            let r = try {
                    Value.Compare(1, 2),
                    Value.Compare(2, 1),
                    Value.Compare(1, 1),
                    Value.Compare("a", "b"),
                    Value.Compare("b", "a")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q389", () =>
            let r = try {
                    Value.Compare(null, 1),
                    Value.Compare(1, null),
                    Value.Compare(null, null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q390", () =>
            let r = try {
                    Value.Compare("a", "A", Comparer.OrdinalIgnoreCase),
                    Value.Compare("A", "a", Comparer.Ordinal),
                    Value.Compare(#date(2024, 1, 1), #date(2024, 6, 1))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q391", () =>
            let r = try Function.Invoke((x as number, y as number) => x + y, {3, 4}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q392", () =>
            let r = try Function.Invoke(Text.Upper, {"hello"}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q393", () =>
            let r = try Function.Invoke(List.Sum, {{1, 2, 3, 4, 5}}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q394", () =>
            let r = try Function.Invoke(Text.Combine, {{"a", "b", "c"}, "-"}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q395", () =>
            let r = try Function.Invoke((x as number) => x * 2, {21}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q396", () =>
            let r = try {
                    Type.Is(type number, type any),
                    Type.Is(type text, type number),
                    Type.Is(type number, type number),
                    Type.Is(type {number}, type list)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q397", () =>
            let r = try Record.FieldNames(Type.RecordFields(type [a = number, b = text, c = logical])) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q398", () =>
            let r = try Record.FieldNames(Type.FunctionParameters(type function (x as number, y as text) as logical)) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q399", () =>
            let r = try Value.Is(42, type number) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q400", () =>
            let r = try {
                    Value.Is(42, type number),
                    Value.Is("hi", type number),
                    Value.Is(null, type number),
                    Value.Is(null, type nullable number),
                    Value.Is({1, 2}, type list)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q401", () =>
            let r = try Expression.Evaluate("1 + 2") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q402", () =>
            let r = try Expression.Evaluate("Text.Upper(""hello"")", #shared) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q403", () =>
            let r = try Expression.Evaluate("x + y", [x = 10, y = 32]) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q404", () =>
            let r = try Expression.Evaluate("not a valid M syntax {{") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q405", () =>
            let r = try Expression.Evaluate("let a = 5, b = 7 in a * b") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q406", () =>
            let r = try Record.FieldNames(Record.AddField([a=1], "b", 2, false)) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q407", () =>
            let r = try Record.FieldNames(Record.AddField([a=1], "b", () => 42, true)) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q408", () =>
            let r = try
                let
                    rec = Record.AddField([a=1], "b", () => 42, true),
                    v = Record.Field(rec, "b"),
                    forced = if Value.Is(v, type function) then v() else v
                in
                    forced
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q409", () =>
            let r = try
                let
                    rec = Record.AddField([], "computed", () => 10 * 3, true),
                    v = Record.Field(rec, "computed"),
                    forced = if Value.Is(v, type function) then v() else v
                in
                    forced
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q410", () =>
            let r = try
                let
                    rec = Record.AddField([], "x", () => error "computed!", true),
                    v = Record.Field(rec, "x"),
                    forced = try (if Value.Is(v, type function) then v() else v)
                in
                    if forced[HasError] then "errored: " & forced[Error][Message] else "no error"
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q411", () =>
            let r = try Record.FromList({1, 2, 3}, {"a", "b", "c"}) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q412", () =>
            let r = try Record.ToList([a=1, b=2, c=3]) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q413", () =>
            let r = try Record.FromTable(#table({"Name", "Value"}, {{"a", 1}, {"b", 2}, {"c", 3}})) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q414", () =>
            let r = try Record.ToTable([a=1, b=2, c=3]) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q415", () =>
            let r = try
                let
                    orig = [x=10, y=20, z=30],
                    asList = Record.ToList(orig),
                    roundtrip = Record.FromList(asList, Record.FieldNames(orig))
                in
                    Record.FieldValues(roundtrip)
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q416", () =>
            let r = try Table.Group(
                    #table({"k", "v"}, {{"a", 1}, {"a", 2}, {"b", 3}, {"b", 4}, {"a", 5}}),
                    {"k"},
                    {{"Sum", each List.Sum([v]), Int64.Type}}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q417", () =>
            let r = try Table.Group(
                    #table({"k", "v"}, {{"a", 1}, {"a", 2}, {"b", 3}}),
                    {"k"},
                    {{"Count", each Table.RowCount(_), Int64.Type}}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q418", () =>
            let r = try Table.Group(
                    #table({"k", "v"}, {{"a", 1}, {"a", 2}, {"b", 3}}),
                    {"k"},
                    {{"Values", each [v], type list}}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q419", () =>
            let r = try Table.Group(
                    #table({"k", "v"}, {}),
                    {"k"},
                    {{"Count", each Table.RowCount(_), Int64.Type}}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q420", () =>
            let r = try Table.Group(
                    #table({"region", "category", "sales"}, {{"N", "X", 10}, {"N", "Y", 20}, {"S", "X", 30}, {"N", "X", 40}}),
                    {"region", "category"},
                    {{"Total", each List.Sum([sales]), Int64.Type}}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q421", () =>
            let r = try Table.Join(
                    #table({"k", "v"}, {{"a", 1}, {"b", 2}, {"c", 3}}),
                    "k",
                    #table({"kr", "w"}, {{"a", 10}, {"b", 20}, {"d", 40}}),
                    "kr",
                    JoinKind.Inner
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q422", () =>
            let r = try Table.Join(
                    #table({"k", "v"}, {{"a", 1}, {"b", 2}, {"c", 3}}),
                    "k",
                    #table({"kr", "w"}, {{"a", 10}, {"b", 20}, {"d", 40}}),
                    "kr",
                    JoinKind.LeftOuter
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q423", () =>
            let r = try Table.Join(
                    #table({"k", "v"}, {{"a", 1}, {"b", 2}, {"c", 3}}),
                    "k",
                    #table({"kr", "w"}, {{"a", 10}, {"b", 20}, {"d", 40}}),
                    "kr",
                    JoinKind.FullOuter
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q424", () =>
            let r = try
                let
                    joined = Table.NestedJoin(
                        #table({"k", "v"}, {{"a", 1}, {"b", 2}}),
                        "k",
                        #table({"k", "w"}, {{"a", 10}, {"a", 20}, {"b", 30}}),
                        "k",
                        "Sub",
                        JoinKind.LeftOuter
                    )
                in
                    Table.ColumnNames(joined)
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q425", () =>
            let r = try Table.Join(
                    #table({"k1", "k2", "v"}, {{"a", 1, "X"}, {"a", 2, "Y"}, {"b", 1, "Z"}}),
                    {"k1", "k2"},
                    #table({"kr1", "kr2", "w"}, {{"a", 1, 100}, {"a", 2, 200}, {"c", 1, 300}}),
                    {"kr1", "kr2"},
                    JoinKind.Inner
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q426", () =>
            let r = try Table.Join(
                    #table({"k", "v"}, {{"a", 1}, {"b", 2}, {"c", 3}}),
                    "k",
                    #table({"kr", "w"}, {{"a", 10}, {"b", 20}, {"d", 40}}),
                    "kr",
                    JoinKind.RightOuter
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q427", () =>
            let r = try Table.Join(
                    #table({"k", "v"}, {{"a", 1}, {"b", 2}, {"c", 3}}),
                    "k",
                    #table({"kr", "w"}, {{"a", 10}, {"b", 20}, {"d", 40}}),
                    "kr",
                    JoinKind.LeftAnti
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q428", () =>
            let r = try Table.Join(
                    #table({"k", "v"}, {{"a", 1}, {"b", 2}, {"c", 3}}),
                    "k",
                    #table({"kr", "w"}, {{"a", 10}, {"b", 20}, {"d", 40}}),
                    "kr",
                    JoinKind.RightAnti
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q429", () =>
            let r = try Table.NestedJoin(
                    #table({"k", "v"}, {{"a", 1}, {"b", 2}, {"c", 3}}),
                    "k",
                    #table({"k", "w"}, {{"a", 10}, {"b", 20}, {"d", 40}}),
                    "k",
                    "Sub",
                    JoinKind.Inner
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=Table.ColumnNames(r[Value])]),

        SafeSerialize("q430", () =>
            let r = try
                let
                    joined = Table.NestedJoin(
                        #table({"k"}, {{"a"}, {"b"}, {"c"}}),
                        "k",
                        #table({"k", "w"}, {{"a", 10}, {"a", 20}, {"b", 30}, {"d", 40}}),
                        "k",
                        "Sub",
                        JoinKind.LeftOuter
                    ),
                    rowCounts = Table.AddColumn(joined, "n", each Table.RowCount([Sub]))
                in
                    Table.SelectColumns(rowCounts, {"k", "n"})
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q431", () =>
            let r = try Table.TransformColumnTypes(
                    #table({"n", "t"}, {{"1", "a"}, {"2", "b"}, {"3", "c"}}),
                    {{"n", Int64.Type}}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q432", () =>
            let r = try Table.TransformColumnTypes(
                    #table({"d"}, {{"2024-01-15"}, {"2024-06-30"}, {"2024-12-31"}}),
                    {{"d", type date}}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q433", () =>
            let r = try Table.TransformColumnTypes(
                    #table({"n"}, {{"1.5"}, {"2.7"}, {"3.14"}}),
                    {{"n", type number}}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q434", () =>
            let r = try Table.TransformColumnTypes(
                    #table({"a", "b", "c"}, {{"1", "true", "2024-01-01"}, {"2", "false", "2024-06-15"}}),
                    {{"a", Int64.Type}, {"b", type logical}, {"c", type date}}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q435", () =>
            let r = try Table.TransformColumnTypes(
                    #table({"n"}, {{"1.234,56"}, {"2.345,67"}}),
                    {{"n", type number}},
                    "de-DE"
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q436", () =>
            let r = try Table.SplitColumn(
                    #table({"full"}, {{"a,b"}, {"c,d"}, {"e,f"}}),
                    "full",
                    Splitter.SplitTextByDelimiter(","),
                    {"first", "second"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q437", () =>
            let r = try Table.SplitColumn(
                    #table({"full"}, {{"a,b,c"}, {"d,e"}, {"f"}}),
                    "full",
                    Splitter.SplitTextByDelimiter(","),
                    {"p1", "p2", "p3"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q438", () =>
            let r = try Table.CombineColumns(
                    #table({"first", "second", "third"}, {{"a", "b", "c"}, {"d", "e", "f"}}),
                    {"first", "second", "third"},
                    Combiner.CombineTextByDelimiter("-"),
                    "joined"
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q439", () =>
            let r = try Table.SplitColumn(
                    #table({"csv"}, {{"a,""b,c"",d"}, {"e,""f,g"",h"}}),
                    "csv",
                    Splitter.SplitTextByDelimiter(",", QuoteStyle.Csv),
                    {"p1", "p2", "p3"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q440", () =>
            let r = try Table.SplitColumn(
                    #table({"by_pos"}, {{"abcdef"}, {"123456"}}),
                    "by_pos",
                    Splitter.SplitTextByLengths({2, 2, 2}),
                    {"p1", "p2", "p3"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q441", () =>
            let r = try {
                    Date.WeekOfMonth(#date(2024, 1, 1)),
                    Date.WeekOfMonth(#date(2024, 1, 15)),
                    Date.WeekOfMonth(#date(2024, 1, 31)),
                    Date.WeekOfMonth(#date(2024, 12, 31))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q442", () =>
            let r = try {
                    Date.WeekOfYear(#date(2024, 1, 1)),
                    Date.WeekOfYear(#date(2024, 6, 15)),
                    Date.WeekOfYear(#date(2024, 12, 31)),
                    Date.WeekOfYear(#date(2024, 1, 1), Day.Monday)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q443", () =>
            let r = try {
                    Date.StartOfWeek(#date(2024, 6, 15)),
                    Date.StartOfWeek(#date(2024, 6, 15), Day.Sunday),
                    Date.StartOfWeek(#date(2024, 6, 15), Day.Monday),
                    Date.StartOfWeek(#date(2024, 6, 17), Day.Monday)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q444", () =>
            let r = try {
                    Date.EndOfWeek(#date(2024, 6, 15)),
                    Date.EndOfWeek(#date(2024, 6, 15), Day.Sunday),
                    Date.EndOfWeek(#date(2024, 6, 15), Day.Monday),
                    Date.EndOfWeek(#date(2024, 12, 29))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q445", () =>
            let r = try {
                    Date.DayOfWeek(#date(2024, 6, 16)),
                    Date.DayOfWeek(#date(2024, 6, 16), Day.Sunday),
                    Date.DayOfWeek(#date(2024, 6, 16), Day.Monday),
                    Date.DayOfWeekName(#date(2024, 6, 16))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q446", () =>
            let r = try {
                    Date.FromText("2024-06-15"),
                    Date.FromText("2024-12-31"),
                    Date.FromText("2024-02-29")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q447", () =>
            let r = try Date.FromText("15/06/2024", [Format="dd/MM/yyyy", Culture="en-GB"]) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q448", () =>
            let r = try Date.FromText("06/15/2024", [Format="MM/dd/yyyy", Culture="en-US"]) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q449", () =>
            let r = try Date.FromText("not a date") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q450", () =>
            let r = try {
                    Date.FromText("June 15, 2024", "en-US"),
                    Date.FromText("15 Juni 2024", "de-DE")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q451", () =>
            let r = try {
                    Time.Hour(#time(14, 30, 45)),
                    Time.Minute(#time(14, 30, 45)),
                    Time.Second(#time(14, 30, 45)),
                    Time.Hour(#time(0, 0, 0)),
                    Time.Hour(#time(23, 59, 59))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q452", () =>
            let r = try {
                    Time.From(#time(14, 30, 45)),
                    Time.From(#datetime(2024, 6, 15, 9, 15, 30)),
                    Time.From(0.5),
                    Time.From(0.75)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q453", () =>
            let r = try {
                    Time.ToText(#time(14, 30, 45)),
                    Time.ToText(#time(0, 0, 0)),
                    Time.ToText(#time(23, 59, 59))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q454", () =>
            let r = try
                let
                    t = #time(14, 30, 45),
                    d = #duration(0, 1, 30, 0),
                    sum = t + d
                in
                    sum
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q455", () =>
            let r = try
                let
                    t1 = #time(14, 30, 0),
                    t2 = #time(16, 45, 30),
                    diff = t2 - t1
                in
                    Duration.TotalMinutes(diff)
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q456", () =>
            let r = try {
                    Logical.From(true),
                    Logical.From(false),
                    Logical.From(1),
                    Logical.From(0),
                    Logical.From(null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q457", () =>
            let r = try {
                    Logical.FromText("true"),
                    Logical.FromText("false"),
                    Logical.FromText("TRUE"),
                    Logical.FromText("False")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q458", () =>
            let r = try {
                    try Logical.FromText("1") otherwise "err",
                    try Logical.FromText("0") otherwise "err",
                    try Logical.FromText("yes") otherwise "err",
                    try Logical.FromText("") otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q459", () =>
            let r = try {
                    Logical.ToText(true),
                    Logical.ToText(false)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q460", () =>
            let r = try {
                    Logical.From(2),
                    Logical.From(-1),
                    Logical.From(0.5),
                    try Logical.From("true") otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q461", () =>
            let r = try {
                    Percentage.From("50%"),
                    Percentage.From("100%"),
                    Percentage.From("0%"),
                    Percentage.From("12.5%"),
                    Percentage.From("-25%")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q462", () =>
            let r = try {
                    Percentage.From(0.5),
                    Percentage.From(1),
                    Percentage.From(null),
                    Percentage.From(true)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q463", () =>
            let r = try Value.Is(0.5, Percentage.Type) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q464", () =>
            let r = try {
                    try Percentage.From("not a percent") otherwise "err",
                    try Percentage.From("50") otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q465", () =>
            let r = try Percentage.From("50,5%", "fr-FR") in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q466", () =>
            let r = try {
                    Currency.From(123.45),
                    Currency.From(0),
                    Currency.From(null),
                    Currency.From(-5.99)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q467", () =>
            let r = try {
                    try Currency.From("123.45") otherwise "err",
                    try Currency.From("$100.50") otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q468", () =>
            let r = try Value.Is(123.45, Currency.Type) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q469", () =>
            let r = try
                let
                    v = Currency.From(123.456789)
                in
                    v
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q470", () =>
            let r = try
                let
                    a = Currency.From(10.5),
                    b = Currency.From(2.25),
                    sum = a + b
                in
                    sum
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q471", () =>
            let r = try
                let
                    a = Decimal.From(0.1),
                    b = Decimal.From(0.2),
                    sum = a + b
                in
                    sum
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q472", () =>
            let r = try {
                    Value.Is(Decimal.From(1.5), Decimal.Type),
                    Value.Is(Decimal.From(1.5), type number),
                    Value.Is(1.5, Decimal.Type),
                    Value.Is(1.5, Double.Type)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q473", () =>
            let r = try
                let
                    d = Decimal.From(1.5),
                    f = 2.0,
                    sum = d + f
                in
                    Value.Is(sum, Decimal.Type)
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q474", () =>
            let r = try {
                    Decimal.From("123.456"),
                    Decimal.From("0.0001"),
                    Decimal.From(null),
                    Decimal.From(-99.99)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q475", () =>
            let r = try {
                    Int64.From(123.7),
                    Int64.From(123.4),
                    Int64.From(-123.7),
                    Int64.From("42")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q476", () =>
            let r = try
                let
                    x = Number.Random()
                in
                    x >= 0 and x < 1
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q477", () =>
            let r = try
                let
                    x = Number.RandomBetween(10, 20)
                in
                    x >= 10 and x <= 20
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q478", () =>
            let r = try
                let
                    samples = List.Transform({1..10}, each Number.Random()),
                    allInRange = List.AllTrue(List.Transform(samples, each _ >= 0 and _ < 1))
                in
                    allInRange
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q479", () =>
            let r = try
                let
                    x = Number.RandomBetween(5, 5)
                in
                    x
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q480", () =>
            let r = try
                let
                    samples = List.Transform({1..20}, each Number.RandomBetween(0, 100)),
                    distinctCount = List.Count(List.Distinct(samples))
                in
                    distinctCount > 1
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q481", () =>
            let r = try {
                    try Number.Mod(10, 0) otherwise "err",
                    try Number.IntegerDivide(10, 0) otherwise "err",
                    try Number.Mod(0, 5) otherwise "err",
                    try Number.IntegerDivide(0, 5) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q482", () =>
            let r = try {
                    Number.Mod(7.5, 2),
                    Number.Mod(10, 2.5),
                    Number.Mod(-7.5, 2),
                    Number.Mod(7.5, -2)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q483", () =>
            let r = try {
                    Number.IntegerDivide(10, 3),
                    Number.IntegerDivide(-10, 3),
                    Number.IntegerDivide(10, -3),
                    Number.IntegerDivide(-10, -3)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q484", () =>
            let r = try {
                    Number.Mod(null, 5),
                    Number.Mod(5, null),
                    Number.IntegerDivide(null, 5),
                    Number.IntegerDivide(5, null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q485", () =>
            let r = try
                let
                    naninf = Number.NaN,
                    pinf = Number.PositiveInfinity
                in
                    {
                        try Number.Mod(naninf, 1) otherwise "err",
                        try Number.Mod(1, pinf) otherwise "err",
                        try Number.IntegerDivide(pinf, 1) otherwise "err"
                    }
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q486", () =>
            let r = try {
                    Text.Insert("hello", 0, "X"),
                    Text.Insert("hello", 5, "X"),
                    Text.Insert("hello", 2, ""),
                    Text.Insert("", 0, "abc")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q487", () =>
            let r = try {
                    Text.Remove("hello world", "l"),
                    Text.Remove("hello world", {"l", "o"}),
                    Text.Remove("hello", "z"),
                    Text.Remove("", "x")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q488", () =>
            let r = try {
                    Text.RemoveRange("hello world", 5),
                    Text.RemoveRange("hello world", 5, 1),
                    Text.RemoveRange("hello", 0, 5),
                    Text.RemoveRange("hello", 2, 0)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q489", () =>
            let r = try {
                    Text.Range("hello world", 6),
                    Text.Range("hello world", 0, 5),
                    Text.Range("hello", 0, 0),
                    Text.Range("hello", 5)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q490", () =>
            let r = try {
                    try Text.Insert("hello", -1, "X") otherwise "err",
                    try Text.Insert("hello", 10, "X") otherwise "err",
                    try Text.Range("hello", 10) otherwise "err",
                    try Text.Range("hello", 0, 100) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q491", () =>
            let r = try {
                    Text.PositionOf("hello world", "l"),
                    Text.PositionOf("hello world", "world"),
                    Text.PositionOf("hello world", "xyz"),
                    Text.PositionOf("hello world", "")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q492", () =>
            let r = try Text.PositionOf("hello world", "l", Occurrence.All) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q493", () =>
            let r = try {
                    Text.PositionOfAny("hello world", {"l", "o"}),
                    Text.PositionOfAny("hello world", {"z", "y"}),
                    Text.PositionOfAny("hello world", {"o", "l"}, Occurrence.All)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q494", () =>
            let r = try Text.PositionOf("Hello World", "world", Occurrence.First, Comparer.OrdinalIgnoreCase) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q495", () =>
            let r = try {
                    Text.Contains("hello world", "world"),
                    Text.Contains("hello world", "xyz"),
                    Text.Contains("Hello World", "world", Comparer.OrdinalIgnoreCase),
                    Text.StartsWith("hello world", "hello"),
                    Text.EndsWith("hello world", "world")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q496", () =>
            let r = try Text.Length(Text.NewGuid()) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q497", () =>
            let r = try
                let
                    g = Text.NewGuid(),
                    parts = Text.Split(g, "-")
                in
                    List.Transform(parts, each Text.Length(_))
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q498", () =>
            let r = try
                let
                    samples = List.Transform({1..5}, each Text.NewGuid()),
                    distinct = List.Distinct(samples)
                in
                    List.Count(distinct)
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q499", () =>
            let r = try
                let
                    g = Text.NewGuid(),
                    lower = Text.Lower(g),
                    isHex = List.AllTrue(List.Transform(Text.ToList(Text.Replace(lower, "-", "")), each Text.Contains("0123456789abcdef", _)))
                in
                    isHex
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q500", () =>
            let r = try Text.Length(Text.Replace(Text.NewGuid(), "-", "")) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]])
    },

    Catalog = Table.FromRecords(cases)
in
    Catalog
