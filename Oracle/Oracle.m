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
                    else [HasError=false, Value=r[Value]])
    },

    Catalog = Table.FromRecords(cases)
in
    Catalog
