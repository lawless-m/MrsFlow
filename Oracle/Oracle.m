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
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q501", () =>
            let r = try {
                    List.MatchesAll({1, 2, 3}, each _ > 0),
                    List.MatchesAll({1, -2, 3}, each _ > 0),
                    List.MatchesAll({}, each _ > 0),
                    List.MatchesAll({1, 1, 1}, each _ = 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q502", () =>
            let r = try {
                    List.MatchesAny({1, 2, 3}, each _ > 2),
                    List.MatchesAny({1, 2, 3}, each _ > 10),
                    List.MatchesAny({}, each _ > 0),
                    List.MatchesAny({"a", "b", "c"}, each _ = "b")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q503", () =>
            let r = try {
                    List.IsEmpty({}),
                    List.IsEmpty({1}),
                    List.IsEmpty({null}),
                    List.IsEmpty({""})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q504", () =>
            let r = try
                let
                    lst = {1, 2, 3, 4, 5},
                    allEven = List.MatchesAll(lst, each Number.Mod(_, 2) = 0),
                    anyEven = List.MatchesAny(lst, each Number.Mod(_, 2) = 0)
                in
                    {allEven, anyEven}
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q505", () =>
            let r = try
                let
                    nullList = {null, null, null},
                    mixedList = {1, null, 3}
                in
                    {
                        List.MatchesAll(nullList, each _ = null),
                        List.MatchesAny(mixedList, each _ = null),
                        List.MatchesAll(mixedList, each _ = null)
                    }
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q506", () =>
            let r = try {
                    List.FirstN({1, 2, 3, 4, 5}, 3),
                    List.FirstN({1, 2, 3, 4, 5}, 0),
                    List.FirstN({1, 2, 3, 4, 5}, 100),
                    List.FirstN({}, 3)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q507", () =>
            let r = try {
                    List.LastN({1, 2, 3, 4, 5}, 2),
                    List.LastN({1, 2, 3, 4, 5}, 100),
                    List.LastN({1, 2, 3, 4, 5}, 0),
                    List.LastN({}, 2)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q508", () =>
            let r = try {
                    List.RemoveFirstN({1, 2, 3, 4, 5}, 2),
                    List.RemoveFirstN({1, 2, 3, 4, 5}, 100),
                    List.RemoveLastN({1, 2, 3, 4, 5}, 2),
                    List.RemoveLastN({1, 2, 3, 4, 5}, 100)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q509", () =>
            let r = try {
                    List.FirstN({1, 2, 3, 4, 5}, each _ < 4),
                    List.FirstN({5, 4, 3, 2, 1}, each _ < 4),
                    List.RemoveFirstN({1, 2, 3, 4, 5}, each _ < 3)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q510", () =>
            let r = try {
                    try List.FirstN({1, 2, 3}, -1) otherwise "err",
                    try List.LastN({1, 2, 3}, -1) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q511", () =>
            let r = try {
                    List.Repeat({1, 2, 3}, 3),
                    List.Repeat({1, 2, 3}, 0),
                    List.Repeat({}, 5),
                    List.Repeat({"x"}, 5)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q512", () =>
            let r = try {
                    try List.Repeat({1}, -1) otherwise "err",
                    List.Repeat({"a", "b"}, 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q513", () =>
            let r = try List.Generate(() => 0, each _ < 5, each _ + 1) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q514", () =>
            let r = try {
                    List.Numbers(1, 5),
                    List.Numbers(0, 10, 2),
                    List.Numbers(10, 5, -1),
                    List.Numbers(1, 0)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q515", () =>
            let r = try
                let
                    dates = List.Dates(#date(2024, 6, 15), 5, #duration(1, 0, 0, 0))
                in
                    dates
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q516", () =>
            let r = try {
                    List.AllTrue({true, true, true}),
                    List.AllTrue({true, false, true}),
                    List.AllTrue({false, false}),
                    List.AllTrue({})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q517", () =>
            let r = try {
                    List.AnyTrue({true, false, false}),
                    List.AnyTrue({false, false, false}),
                    List.AnyTrue({true, true, true}),
                    List.AnyTrue({})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q518", () =>
            let r = try {
                    try List.AllTrue({true, null, true}) otherwise "err",
                    try List.AnyTrue({null, false}) otherwise "err",
                    try List.AllTrue({1, 2, 3}) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q519", () =>
            let r = try
                let
                    nums = {1, 2, 3, 4, 5},
                    checks = List.Transform(nums, each _ > 0)
                in
                    List.AllTrue(checks)
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q520", () =>
            let r = try
                let
                    a = 5,
                    b = 10,
                    checks = {a > 0, b > 0, a < b}
                in
                    {List.AllTrue(checks), List.AnyTrue(checks)}
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q521", () =>
            let r = try {
                    List.Distinct({1, 2, 2, 3, 3, 3, 4}),
                    List.Distinct({"a", "A", "b"}),
                    List.Distinct({"a", "A", "b"}, Comparer.OrdinalIgnoreCase),
                    List.Distinct({})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q522", () =>
            let r = try {
                    List.Union({{1, 2, 3}, {3, 4, 5}}),
                    List.Union({{1, 2}, {3, 4}, {5, 6}}),
                    List.Union({{}, {1}}),
                    List.Union({})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q523", () =>
            let r = try {
                    List.Intersect({{1, 2, 3, 4}, {2, 3, 5}}),
                    List.Intersect({{1, 2, 3}, {4, 5, 6}}),
                    List.Intersect({{1, 2}, {1, 2}, {1, 2}}),
                    List.Intersect({{1, 2, 3}, {}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q524", () =>
            let r = try {
                    List.Difference({1, 2, 3, 4, 5}, {2, 4}),
                    List.Difference({1, 2, 3}, {}),
                    List.Difference({}, {1, 2, 3}),
                    List.Difference({"a", "B", "c"}, {"A", "C"}, Comparer.OrdinalIgnoreCase)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q525", () =>
            let r = try List.Union({{1, 2, 3}, {2, 3, 4}, {3, 4, 5}}, Comparer.Ordinal) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q526", () =>
            let r = try {
                    List.Skip({1, 2, 3, 4, 5}, 2),
                    List.Skip({1, 2, 3, 4, 5}, 0),
                    List.Skip({1, 2, 3, 4, 5}, 100),
                    List.Skip({}, 3)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q527", () =>
            let r = try {
                    List.Skip({1, 2, 3, 4, 5}, each _ < 3),
                    List.Skip({5, 4, 3, 2, 1}, each _ < 3),
                    List.Skip({1, 2, 3}, each _ < 100)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q528", () =>
            let r = try {
                    List.Range({1, 2, 3, 4, 5}, 1, 3),
                    List.Range({1, 2, 3, 4, 5}, 0, 5),
                    List.Range({1, 2, 3, 4, 5}, 2, 0),
                    List.Range({1, 2, 3, 4, 5}, 2)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q529", () =>
            let r = try {
                    try List.Range({1, 2, 3, 4, 5}, 10, 3) otherwise "err",
                    try List.Range({1, 2, 3, 4, 5}, 0, 100) otherwise "err",
                    try List.Range({1, 2, 3, 4, 5}, -1, 2) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q530", () =>
            let r = try
                let
                    big = {1..100},
                    window = List.Range(big, 45, 10)
                in
                    window
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q531", () =>
            let r = try {
                    Character.FromNumber(65),
                    Character.FromNumber(97),
                    Character.FromNumber(48),
                    Character.FromNumber(32)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q532", () =>
            let r = try {
                    Character.ToNumber("A"),
                    Character.ToNumber("a"),
                    Character.ToNumber("0"),
                    Character.ToNumber(" ")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q533", () =>
            let r = try
                let
                    roundtrip = Character.FromNumber(Character.ToNumber("Z"))
                in
                    roundtrip
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q534", () =>
            let r = try {
                    Character.FromNumber(8364),
                    Character.FromNumber(233),
                    Character.FromNumber(9731),
                    Character.ToNumber("€")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q535", () =>
            let r = try {
                    try Character.FromNumber(-1) otherwise "err",
                    try Character.FromNumber(1114112) otherwise "err",
                    try Character.ToNumber("") otherwise "err",
                    try Character.ToNumber("ab") otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q536", () =>
            let r = try Text.FromBinary(Json.FromValue([a=1, b=2, c=3]), TextEncoding.Utf8) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q537", () =>
            let r = try Text.FromBinary(Json.FromValue([
                    name = "alpha",
                    nested = [a=1, b=[x=10, y=20]],
                    items = {1, 2, 3}
                ]), TextEncoding.Utf8) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q538", () =>
            let r = try Text.FromBinary(Json.FromValue({{1, 2}, {3, 4}, {5, 6}}), TextEncoding.Utf8) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q539", () =>
            let r = try Text.FromBinary(Json.FromValue([
                    empty_list = {},
                    empty_rec = [],
                    nullable = null,
                    bools = {true, false, true}
                ]), TextEncoding.Utf8) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q540", () =>
            let r = try Text.FromBinary(Json.FromValue([
                    quote = "he said ""hi""",
                    backslash = "C:\path\file",
                    tab = "a#(tab)b",
                    newline = "line1#(lf)line2"
                ]), TextEncoding.Utf8) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q541", () =>
            let r = try {
                    Number.Sign(5),
                    Number.Sign(-3),
                    Number.Sign(0),
                    Number.Abs(-7.5),
                    Number.Abs(7.5)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q542", () =>
            let r = try {
                    Number.Sqrt(16),
                    Number.Sqrt(2),
                    Number.Sqrt(0),
                    try Number.Sqrt(-1) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q543", () =>
            let r = try {
                    Number.Power(2, 10),
                    Number.Power(10, -2),
                    Number.Power(0, 0),
                    Number.Power(-2, 3)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q544", () =>
            let r = try {
                    Number.Exp(1),
                    Number.Ln(1),
                    Number.Ln(Number.E),
                    Number.Log10(100),
                    Number.Log10(1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q545", () =>
            let r = try {
                    try Number.Ln(0) otherwise "err",
                    try Number.Ln(-1) otherwise "err",
                    try Number.Log10(0) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q546", () =>
            let r = try {
                    Number.Sin(0),
                    Number.Cos(0),
                    Number.Tan(0),
                    Number.Sin(1.5707963267948966)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q547", () =>
            let r = try {
                    Number.Asin(0),
                    Number.Asin(1),
                    Number.Acos(1),
                    Number.Acos(0),
                    Number.Atan(0),
                    Number.Atan(1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q548", () =>
            let r = try {
                    Number.Atan2(1, 1),
                    Number.Atan2(1, 0),
                    Number.Atan2(0, -1),
                    Number.Atan2(-1, -1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q549", () =>
            let r = try {
                    try Number.Asin(2) otherwise "err",
                    try Number.Acos(-2) otherwise "err",
                    try Number.Tan(1.5707963267948966) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q550", () =>
            let r = try
                let
                    sin_sq = Number.Power(Number.Sin(0.5), 2),
                    cos_sq = Number.Power(Number.Cos(0.5), 2),
                    identity = sin_sq + cos_sq
                in
                    Number.Abs(identity - 1) < 0.0000001
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q551", () =>
            let r = try Number.Round(Number.PI, 10) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q552", () =>
            let r = try Number.Round(2 * Number.PI, 8) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q553", () =>
            let r = try Number.Round(Number.PI / 4, 10) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q554", () =>
            let r = try Number.PI > 3 and Number.PI < 4 in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q555", () =>
            let r = try Number.Round(Number.Sin(Number.PI / 2), 10) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q556", () =>
            let r = try {
                    Number.BitwiseAnd(12, 10),
                    Number.BitwiseAnd(255, 240),
                    Number.BitwiseAnd(0, 0),
                    Number.BitwiseAnd(-1, 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q557", () =>
            let r = try {
                    Number.BitwiseOr(12, 10),
                    Number.BitwiseOr(0, 255),
                    Number.BitwiseOr(240, 15)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q558", () =>
            let r = try {
                    Number.BitwiseXor(12, 10),
                    Number.BitwiseXor(255, 255),
                    Number.BitwiseXor(0, 255)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q559", () =>
            let r = try {
                    Number.BitwiseShiftLeft(1, 4),
                    Number.BitwiseShiftLeft(3, 2),
                    Number.BitwiseShiftRight(256, 4),
                    Number.BitwiseShiftRight(255, 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q560", () =>
            let r = try
                let
                    a = 12,
                    b = 10,
                    sum_via_bitwise = Number.BitwiseOr(Number.BitwiseAnd(a, b), Number.BitwiseXor(a, b))
                in
                    sum_via_bitwise
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q561", () =>
            let r = try {
                    Text.Proper("hello world"),
                    Text.Proper("HELLO WORLD"),
                    Text.Proper("hELLo wORLD"),
                    Text.Proper("a")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q562", () =>
            let r = try {
                    Text.Trim("  hello  "),
                    Text.Trim("hello"),
                    Text.Trim("   "),
                    Text.Trim("#(tab)hello#(lf)")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q563", () =>
            let r = try {
                    Text.TrimStart("  hello  "),
                    Text.TrimEnd("  hello  "),
                    Text.TrimStart("xxhelloxx", "x"),
                    Text.TrimEnd("xxhelloxx", "x")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q564", () =>
            let r = try {
                    Text.Trim("abcxyz", {"a", "z"}),
                    Text.TrimStart("abcabc", {"a", "b"}),
                    Text.TrimEnd("xxyyzz", {"y", "z"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q565", () =>
            let r = try {
                    Text.Trim(""),
                    Text.Proper(""),
                    Text.Trim("hello", "h"),
                    Text.Trim("aaa", "a")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q566", () =>
            let r = try {
                    Text.Lower("HELLO WORLD"),
                    Text.Lower("Hello World"),
                    Text.Lower(""),
                    Text.Upper("hello world"),
                    Text.Upper("aBcDeF")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q567", () =>
            let r = try {
                    try Text.Lower("IZMIR", "tr-TR") otherwise "err",
                    try Text.Lower("HELLO", "en-US") otherwise "err",
                    try Text.Upper("istanbul", "tr-TR") otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q568", () =>
            let r = try {
                    Text.Lower("ÄÖÜß"),
                    Text.Upper("äöüß"),
                    Text.Lower("ÉÈÊË"),
                    Text.Upper("éèêë")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q569", () =>
            let r = try {
                    Text.Length(Text.Lower("HELLO")),
                    Text.Length(Text.Upper("hello")),
                    Text.Lower("hello") = "hello",
                    Text.Upper("HELLO") = "HELLO"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q570", () =>
            let r = try {
                    Text.Length("ÄÖÜß"),
                    Text.Length("hello"),
                    Character.ToNumber("Ä"),
                    Character.ToNumber("ß")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q571", () =>
            let r = try {
                    List.Transform({1, 2, 3}, each _ * 2),
                    List.Transform({1, 2, 3}, (x) => x * 2),
                    List.Transform({1, 2, 3}, Number.Sqrt),
                    List.Transform({}, each _)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q572", () =>
            let r = try
                let
                    multiplyBy = (factor) => (x) => x * factor,
                    triple = multiplyBy(3)
                in
                    List.Transform({1, 2, 3, 4}, triple)
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q573", () =>
            let r = try List.Transform({1..5}, each _ * _) in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q574", () =>
            let r = try
                let
                    pairs = {{1, 2}, {3, 4}, {5, 6}}
                in
                    List.Transform(pairs, each _{0} + _{1})
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q575", () =>
            let r = try
                let
                    adder = (a) => (b) => a + b,
                    add5 = adder(5),
                    add10 = adder(10),
                    applied = List.Transform({1, 2, 3}, add5)
                in
                    applied
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q576", () =>
            let r = try {
                    Record.RemoveFields([a=1, b=2, c=3], "b"),
                    Record.RemoveFields([a=1, b=2, c=3], {"a", "c"}),
                    Record.RemoveFields([a=1, b=2, c=3], {})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q577", () =>
            let r = try {
                    Record.RenameFields([a=1, b=2, c=3], {{"a", "alpha"}}),
                    Record.RenameFields([a=1, b=2, c=3], {{"a", "x"}, {"c", "z"}}),
                    Record.RenameFields([a=1, b=2], {})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q578", () =>
            let r = try {
                    Record.TransformFields([a=1, b=2, c=3], {{"a", each _ * 10}}),
                    Record.TransformFields([n=5], {{"n", Text.From}}),
                    Record.TransformFields([a=1, b=2], {{"a", each _ + 100}, {"b", each _ - 1}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q579", () =>
            let r = try {
                    try Record.RemoveFields([a=1, b=2], "z") otherwise "err",
                    try Record.RenameFields([a=1], {{"x", "y"}}) otherwise "err",
                    try Record.TransformFields([a=1], {{"z", each _ * 2}}) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q580", () =>
            let r = try
                let
                    original = [a=1, b=2, c=3, d=4],
                    step1 = Record.RemoveFields(original, "d"),
                    step2 = Record.RenameFields(step1, {{"a", "alpha"}, {"c", "charlie"}}),
                    step3 = Record.TransformFields(step2, {{"alpha", each _ * 100}})
                in
                    step3
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q581", () =>
            let r = try Table.Distinct(
                    #table({"k", "v"}, {{"a", 1}, {"b", 2}, {"a", 1}, {"c", 3}, {"b", 2}})
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q582", () =>
            let r = try Table.Distinct(
                    #table({"k", "v"}, {{"a", 1}, {"a", 2}, {"b", 3}, {"a", 1}}),
                    "k"
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q583", () =>
            let r = try Table.Distinct(
                    #table({"a", "b", "c"}, {{"x", 1, 10}, {"x", 1, 20}, {"y", 1, 10}}),
                    {"a", "b"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q584", () =>
            let r = try Table.Distinct(
                    #table({"k"}, {{"A"}, {"a"}, {"B"}, {"b"}}),
                    {"k", Comparer.OrdinalIgnoreCase}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q585", () =>
            let r = try {
                    Table.RowCount(Table.Distinct(#table({"k"}, {{"a"}, {"a"}, {"a"}}))),
                    Table.RowCount(Table.Distinct(#table({"k"}, {}))),
                    Table.RowCount(Table.Distinct(#table({"k"}, {{"a"}, {"b"}, {"c"}})))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q586", () =>
            let r = try Table.Sort(
                    #table({"n"}, {{3}, {1}, {2}, {5}, {4}}),
                    "n"
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q587", () =>
            let r = try Table.Sort(
                    #table({"n"}, {{3}, {1}, {2}, {5}, {4}}),
                    {"n", Order.Descending}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q588", () =>
            let r = try Table.Sort(
                    #table({"g", "v"}, {{"a", 3}, {"b", 1}, {"a", 1}, {"b", 2}, {"a", 2}}),
                    {{"g", Order.Ascending}, {"v", Order.Ascending}}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q589", () =>
            let r = try Table.Sort(
                    #table({"g", "v"}, {{"a", 3}, {"b", 1}, {"a", 1}, {"b", 2}, {"a", 2}}),
                    {{"g", Order.Ascending}, {"v", Order.Descending}}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q590", () =>
            let r = try Table.Sort(
                    #table({"a", "b"}, {{1, 1}, {1, 2}, {1, 1}, {2, 1}, {1, 2}}),
                    "a"
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q591", () =>
            let r = try Table.SelectRows(
                    #table({"n"}, {{1}, {2}, {3}, {4}, {5}}),
                    each [n] > 2
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q592", () =>
            let r = try Table.SelectRows(
                    #table({"a", "b"}, {{1, 10}, {2, 20}, {3, 30}, {4, 40}, {5, 50}}),
                    each [a] > 1 and [b] < 40
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q593", () =>
            let r = try Table.SelectRows(
                    #table({"name", "score"}, {{"Alice", 85}, {"Bob", 72}, {"Charlie", 91}, {"Dave", 67}}),
                    each Text.StartsWith([name], "A") or [score] > 80
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q594", () =>
            let r = try Table.SelectRows(
                    #table({"v"}, {{1}, {null}, {3}, {null}, {5}}),
                    each [v] <> null
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q595", () =>
            let r = try Table.SelectRows(
                    #table({"n"}, {{1}, {2}, {3}, {4}, {5}}),
                    (row) => Number.Mod(row[n], 2) = 0
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q596", () =>
            let r = try Table.FillDown(
                    #table({"a", "b"}, {{"X", 1}, {null, 2}, {null, 3}, {"Y", 4}, {null, 5}}),
                    {"a"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q597", () =>
            let r = try Table.FillUp(
                    #table({"a", "b"}, {{null, 1}, {null, 2}, {"X", 3}, {null, 4}, {"Y", 5}}),
                    {"a"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q598", () =>
            let r = try Table.FillDown(
                    #table({"a", "b", "c"}, {{"X", null, 1}, {null, "Q", 2}, {null, null, 3}}),
                    {"a", "b"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q599", () =>
            let r = try Table.FillDown(
                    #table({"a"}, {{null}, {null}, {"X"}, {null}}),
                    {"a"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q600", () =>
            let r = try Table.FillDown(
                    #table({"a"}, {{"X"}, {"Y"}, {"Z"}}),
                    {"a"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q601", () =>
            let r = try Table.ReplaceValue(
                    #table({"v"}, {{"hello"}, {"world"}, {"hello"}}),
                    "hello",
                    "HI",
                    Replacer.ReplaceValue,
                    {"v"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q602", () =>
            let r = try Table.ReplaceValue(
                    #table({"v"}, {{"foo bar"}, {"bar baz"}, {"qux"}}),
                    "bar",
                    "X",
                    Replacer.ReplaceText,
                    {"v"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q603", () =>
            let r = try Table.ReplaceValue(
                    #table({"a", "b"}, {{1, 1}, {1, 2}, {2, 1}}),
                    1,
                    99,
                    Replacer.ReplaceValue,
                    {"a"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q604", () =>
            let r = try Table.ReplaceValue(
                    #table({"v"}, {{null}, {"x"}, {null}, {"y"}}),
                    null,
                    "MISSING",
                    Replacer.ReplaceValue,
                    {"v"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q605", () =>
            let r = try Table.ReplaceErrorValues(
                    Table.AddColumn(
                        #table({"n"}, {{2}, {0}, {4}, {0}, {8}}),
                        "inv",
                        each if [n] = 0 then error "div by zero" else 100 / [n]
                    ),
                    {{"inv", -1}}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q606", () =>
            let r = try {
                    Date.AddDays(#date(2024, 6, 15), 10),
                    #date(2024, 6, 15) + #duration(7, 0, 0, 0),
                    #date(2024, 6, 15) - #duration(1, 0, 0, 0)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q607", () =>
            let r = try
                let
                    d = #date(2024, 6, 30) - #date(2024, 6, 15)
                in
                    {Duration.TotalDays(d), Duration.TotalHours(d), Duration.TotalMinutes(d), Duration.TotalSeconds(d)}
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q608", () =>
            let r = try
                let
                    d = #duration(1, 2, 30, 45)
                in
                    {Duration.Days(d), Duration.Hours(d), Duration.Minutes(d), Duration.Seconds(d), Duration.TotalSeconds(d)}
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q609", () =>
            let r = try
                let
                    negDur = #duration(-1, 0, 0, 0),
                    dur2 = #duration(0, 25, 0, 0)
                in
                    {Duration.TotalDays(negDur), Duration.TotalHours(dur2), Duration.Days(dur2), Duration.Hours(dur2)}
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q610", () =>
            let r = try
                let
                    dt = #datetime(2024, 6, 15, 10, 30, 0),
                    later = dt + #duration(0, 5, 30, 0)
                in
                    later
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q611", () =>
            let r = try {
                    DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 45), "yyyy-MM-dd HH:mm:ss"),
                    DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 45), "yyyy/MM/dd"),
                    DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 45), "dd-MMM-yyyy"),
                    DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 45), "HH:mm")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q612", () =>
            let r = try {
                    DateTime.ToText(#datetime(2024, 1, 5, 9, 5, 7), "yyyy-MM-dd"),
                    DateTime.ToText(#datetime(2024, 1, 5, 9, 5, 7), "yyyy-M-d"),
                    DateTime.ToText(#datetime(2024, 1, 5, 9, 5, 7), "H:m:s"),
                    DateTime.ToText(#datetime(2024, 1, 5, 9, 5, 7), "HH:mm:ss")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q613", () =>
            let r = try {
                    DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 0), "dddd"),
                    DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 0), "ddd"),
                    DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 0), "MMMM"),
                    DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 0), "MMM")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q614", () =>
            let r = try {
                    DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 0), "tt"),
                    DateTime.ToText(#datetime(2024, 6, 15, 9, 30, 0), "h:mm tt"),
                    DateTime.ToText(#datetime(2024, 6, 15, 23, 59, 0), "h:mm tt"),
                    DateTime.ToText(#datetime(2024, 6, 15, 0, 0, 0), "h:mm tt")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q615", () =>
            let r = try {
                    DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 45), "yyyy-MM-dd'T'HH:mm:ss"),
                    DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 45), "yyyyMMdd"),
                    DateTime.ToText(#datetime(2024, 6, 15, 14, 30, 45), "yyyy-DDD")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q616", () =>
            let r = try {
                    Date.AddDays(#date(2024, 12, 31), 1),
                    Date.AddDays(#date(2024, 12, 31), 365),
                    Date.AddDays(#date(2025, 1, 1), -1),
                    Date.AddDays(#date(2024, 1, 1), -1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q617", () =>
            let r = try {
                    Date.AddYears(#date(2024, 2, 29), 1),
                    Date.AddYears(#date(2024, 2, 29), 4),
                    Date.AddYears(#date(2020, 2, 29), -100),
                    Date.AddMonths(#date(2024, 1, 31), 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q618", () =>
            let r = try {
                    Date.IsLeapYear(#date(2024, 1, 1)),
                    Date.IsLeapYear(#date(2023, 1, 1)),
                    Date.IsLeapYear(#date(2000, 1, 1)),
                    Date.IsLeapYear(#date(1900, 1, 1)),
                    Date.IsLeapYear(#date(2100, 1, 1))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q619", () =>
            let r = try {
                    Date.DaysInMonth(#date(2024, 2, 1)),
                    Date.DaysInMonth(#date(2023, 2, 1)),
                    Date.DaysInMonth(#date(2024, 1, 1)),
                    Date.DaysInMonth(#date(2024, 4, 1))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q620", () =>
            let r = try {
                    Date.AddQuarters(#date(2024, 11, 15), 1),
                    Date.AddQuarters(#date(2024, 11, 15), 2),
                    Date.AddWeeks(#date(2024, 12, 25), 2),
                    Date.DayOfYear(#date(2024, 12, 31)),
                    Date.DayOfYear(#date(2023, 12, 31))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q621", () =>
            let r = try Table.AddIndexColumn(
                    #table({"k"}, {{"a"}, {"b"}, {"c"}}),
                    "idx"
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q622", () =>
            let r = try Table.AddIndexColumn(
                    #table({"k"}, {{"a"}, {"b"}, {"c"}}),
                    "idx",
                    1
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q623", () =>
            let r = try Table.AddIndexColumn(
                    #table({"k"}, {{"a"}, {"b"}, {"c"}, {"d"}}),
                    "idx",
                    10,
                    5
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q624", () =>
            let r = try Table.AddIndexColumn(
                    #table({"k"}, {}),
                    "idx",
                    0
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q625", () =>
            let r = try Table.AddIndexColumn(
                    #table({"k"}, {{"x"}, {"y"}, {"z"}}),
                    "idx",
                    5,
                    -1
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q626", () =>
            let r = try Table.PromoteHeaders(
                    #table({"Column1", "Column2", "Column3"}, {{"a", "b", "c"}, {"1", "2", "3"}})
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q627", () =>
            let r = try Table.DemoteHeaders(
                    #table({"a", "b", "c"}, {{1, 2, 3}, {4, 5, 6}})
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q628", () =>
            let r = try
                let
                    t1 = #table({"a", "b"}, {{"x", "y"}, {1, 2}}),
                    demoted = Table.DemoteHeaders(t1),
                    roundtrip = Table.PromoteHeaders(demoted)
                in
                    roundtrip
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q629", () =>
            let r = try Table.PromoteHeaders(
                    #table({"Column1"}, {})
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=Table.ColumnNames(r[Value])]),

        SafeSerialize("q630", () =>
            let r = try
                let
                    t = #table({"Column1", "Column2"}, {{"a", "b"}, {"c", "d"}, {"e", "f"}}),
                    promoted = Table.PromoteHeaders(t)
                in
                    Table.ColumnNames(promoted)
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        SafeSerialize("q631", () =>
            let r = try Table.ReorderColumns(
                    #table({"a", "b", "c", "d"}, {{1, 2, 3, 4}}),
                    {"c", "a", "d", "b"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q632", () =>
            let r = try Table.RemoveColumns(
                    #table({"a", "b", "c"}, {{1, 2, 3}, {4, 5, 6}}),
                    {"b"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q633", () =>
            let r = try Table.Column(
                    #table({"a", "b", "c"}, {{1, 2, 3}, {4, 5, 6}, {7, 8, 9}}),
                    "b"
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q634", () =>
            let r = try Table.SelectColumns(
                    #table({"a", "b", "c", "d"}, {{1, 2, 3, 4}}),
                    {"c", "a"}
                ) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q635", () =>
            let r = try {
                    try Table.RemoveColumns(#table({"a", "b"}, {{1, 2}}), {"x"}) otherwise "err",
                    try Table.SelectColumns(#table({"a", "b"}, {{1, 2}}), {"x"}) otherwise "err",
                    try Table.Column(#table({"a"}, {{1}}), "x") otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q636", () =>
            let r = try {
                    Table.RowCount(#table({"a"}, {{1}, {2}, {3}})),
                    Table.RowCount(#table({"a"}, {})),
                    Table.ColumnCount(#table({"a", "b", "c"}, {{1, 2, 3}})),
                    Table.ColumnCount(#table({}, {}))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q637", () =>
            let r = try {
                    Table.First(#table({"v"}, {{1}, {2}, {3}})),
                    Table.Last(#table({"v"}, {{1}, {2}, {3}})),
                    Table.First(#table({"v"}, {})),
                    Table.Last(#table({"v"}, {}))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q638", () =>
            let r = try {
                    Table.Min(#table({"v"}, {{3}, {1}, {2}}), "v"),
                    Table.Max(#table({"v"}, {{3}, {1}, {2}}), "v")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q639", () =>
            let r = try {
                    Table.RowCount(#table({"a", "b"}, {{1, 2}, {3, 4}, {5, 6}, {7, 8}, {9, 10}})),
                    Table.ColumnCount(#table({"a", "b"}, {{1, 2}, {3, 4}})),
                    Table.RowCount(#table({}, {})),
                    Table.ColumnCount(#table({"only"}, {{1}}))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q640", () =>
            let r = try {
                    Table.IsEmpty(#table({"a"}, {})),
                    Table.IsEmpty(#table({"a"}, {{1}})),
                    Table.IsEmpty(#table({}, {}))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q641", () =>
            let r = try {
                    List.Sum({1, 2, 3, 4, 5}),
                    List.Sum({}),
                    List.Sum({1.5, 2.5, 3.0}),
                    List.Sum({1, null, 3})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q642", () =>
            let r = try {
                    List.Average({1, 2, 3, 4, 5}),
                    List.Average({10, 20, 30}),
                    List.Average({}),
                    List.Average({1, null, 3})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q643", () =>
            let r = try {
                    List.Median({1, 2, 3, 4, 5}),
                    List.Median({1, 2, 3, 4}),
                    List.Median({5, 1, 4, 2, 3}),
                    List.Median({1})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q644", () =>
            let r = try {
                    List.Mode({1, 2, 2, 3, 3, 3, 4}),
                    List.Mode({"a", "b", "a", "c", "a"}),
                    try List.Mode({1, 2, 3}) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q645", () =>
            let r = try {
                    Number.Round(List.StandardDeviation({2, 4, 4, 4, 5, 5, 7, 9}), 6),
                    List.Min({3, 1, 4, 1, 5, 9}),
                    List.Max({3, 1, 4, 1, 5, 9}),
                    List.Count({1, 2, 3, 4, 5}),
                    List.Count({})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q646", () =>
            let r = try {
                    List.RemoveItems({1, 2, 3, 4, 5}, {2, 4}),
                    List.RemoveItems({1, 2, 3}, {}),
                    List.RemoveItems({"a", "b", "c"}, {"x"}),
                    List.RemoveItems({}, {1, 2})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q647", () =>
            let r = try {
                    List.RemoveMatchingItems({1, 2, 3, 1, 2}, {1, 2}),
                    List.RemoveMatchingItems({"a", "b", "a", "c", "a"}, {"a"}),
                    List.RemoveMatchingItems({1, 2, 3}, {})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q648", () =>
            let r = try {
                    List.RemoveNulls({1, null, 2, null, 3}),
                    List.RemoveNulls({null, null}),
                    List.RemoveNulls({1, 2, 3}),
                    List.RemoveNulls({})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q649", () =>
            let r = try {
                    List.Reverse({1, 2, 3, 4, 5}),
                    List.Reverse({}),
                    List.Reverse({"a"}),
                    List.Reverse(List.Reverse({1, 2, 3}))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q650", () =>
            let r = try {
                    List.Sort({3, 1, 4, 1, 5, 9, 2, 6}),
                    List.Sort({"banana", "apple", "cherry"}),
                    List.Sort({}),
                    List.Sort({3, 1, 4, 1, 5, 9, 2, 6}, Order.Descending)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q651", () =>
            let r = try {
                    Text.Length("hello"),
                    Text.Length(""),
                    Text.Length("a"),
                    Text.Length("hello world")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q652", () =>
            let r = try {
                    Text.Start("hello world", 5),
                    Text.Start("hi", 5),
                    Text.Start("", 3),
                    Text.Start("hello", 0)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q653", () =>
            let r = try {
                    Text.End("hello world", 5),
                    Text.End("hi", 5),
                    Text.End("", 3),
                    Text.End("hello", 0)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q654", () =>
            let r = try {
                    Text.Middle("hello world", 6, 5),
                    Text.Middle("hello", 1, 3),
                    Text.Middle("hello", 0, 5),
                    Text.Middle("hello", 2)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q655", () =>
            let r = try {
                    Text.At("hello", 0),
                    Text.At("hello", 4),
                    try Text.At("hello", 10) otherwise "err",
                    try Text.At("", 0) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q656-q662: Number.Round corner sweep — RoundingMode × digit counts ×
        // signed inputs × Inf/NaN. Phase 2 depth probes.

        SafeSerialize("q656", () =>
            let r = try {
                    Number.Round(0.5, 0, RoundingMode.AwayFromZero),
                    Number.Round(1.5, 0, RoundingMode.AwayFromZero),
                    Number.Round(2.5, 0, RoundingMode.AwayFromZero),
                    Number.Round(-0.5, 0, RoundingMode.AwayFromZero),
                    Number.Round(-1.5, 0, RoundingMode.AwayFromZero),
                    Number.Round(-2.5, 0, RoundingMode.AwayFromZero)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q657", () =>
            let r = try {
                    Number.Round(0.5, 0, RoundingMode.ToEven),
                    Number.Round(1.5, 0, RoundingMode.ToEven),
                    Number.Round(2.5, 0, RoundingMode.ToEven),
                    Number.Round(3.5, 0, RoundingMode.ToEven),
                    Number.Round(-0.5, 0, RoundingMode.ToEven),
                    Number.Round(-1.5, 0, RoundingMode.ToEven),
                    Number.Round(-2.5, 0, RoundingMode.ToEven)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q658", () =>
            let r = try {
                    Number.Round(1.25, 1, RoundingMode.AwayFromZero),
                    Number.Round(1.25, 1, RoundingMode.ToEven),
                    Number.Round(1.25, 1, RoundingMode.Down),
                    Number.Round(1.25, 1, RoundingMode.Up),
                    Number.Round(1.25, 1, RoundingMode.TowardZero),
                    Number.Round(-1.25, 1, RoundingMode.Down),
                    Number.Round(-1.25, 1, RoundingMode.Up),
                    Number.Round(-1.25, 1, RoundingMode.TowardZero)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q659", () =>
            let r = try {
                    Number.Round(1234.5678, -2, RoundingMode.AwayFromZero),
                    Number.Round(1234.5678, -1, RoundingMode.AwayFromZero),
                    Number.Round(1234.5678, 0, RoundingMode.AwayFromZero),
                    Number.Round(1234.5678, 1, RoundingMode.AwayFromZero),
                    Number.Round(1234.5678, 2, RoundingMode.AwayFromZero),
                    Number.Round(1234.5678, 6, RoundingMode.AwayFromZero),
                    Number.Round(0.000123456, 4, RoundingMode.AwayFromZero),
                    Number.Round(0.000123456, 8, RoundingMode.AwayFromZero)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q660", () =>
            let r = try {
                    Number.Round(0, 0, RoundingMode.AwayFromZero),
                    Number.Round(0, 0, RoundingMode.ToEven),
                    Number.Round(0.0, 2, RoundingMode.Down),
                    Number.Round(-0.0, 0, RoundingMode.AwayFromZero),
                    Number.Round(1, 0, RoundingMode.ToEven),
                    Number.Round(-1, 0, RoundingMode.ToEven),
                    Number.Round(100, -2, RoundingMode.AwayFromZero),
                    Number.Round(100, -3, RoundingMode.AwayFromZero)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q661", () =>
            let r = try {
                    try Number.Round(Number.NaN, 0, RoundingMode.AwayFromZero) otherwise "err",
                    try Number.Round(Number.PositiveInfinity, 0, RoundingMode.AwayFromZero) otherwise "err",
                    try Number.Round(Number.NegativeInfinity, 0, RoundingMode.AwayFromZero) otherwise "err",
                    try Number.Round(Number.NaN, 0, RoundingMode.ToEven) otherwise "err",
                    try Number.Round(Number.PositiveInfinity, 2, RoundingMode.ToEven) otherwise "err",
                    try Number.Round(null, 0, RoundingMode.ToEven) otherwise "err",
                    try Number.Round(1.5, null, RoundingMode.ToEven) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q662", () =>
            let r = try {
                    Number.Round(1234.5, 0, RoundingMode.Up),
                    Number.Round(1234.5, 0, RoundingMode.Down),
                    Number.Round(-1234.5, 0, RoundingMode.Up),
                    Number.Round(-1234.5, 0, RoundingMode.Down),
                    Number.Round(0.5, 0, RoundingMode.TowardZero),
                    Number.Round(-0.5, 0, RoundingMode.TowardZero),
                    Number.Round(2.675, 2, RoundingMode.ToEven),
                    Number.Round(2.675, 2, RoundingMode.AwayFromZero)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q663-q669: Number.Mod sign matrix.

        SafeSerialize("q663", () =>
            let r = try {
                    Number.Mod(7, 3),
                    Number.Mod(-7, 3),
                    Number.Mod(7, -3),
                    Number.Mod(-7, -3),
                    Number.Mod(0, 3),
                    Number.Mod(0, -3),
                    Number.Mod(3, 3),
                    Number.Mod(-3, 3)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q664", () =>
            let r = try {
                    Number.Mod(7.5, 2),
                    Number.Mod(-7.5, 2),
                    Number.Mod(7.5, -2),
                    Number.Mod(-7.5, -2),
                    Number.Mod(2.5, 0.5),
                    Number.Mod(-2.5, 0.5),
                    Number.Mod(2.5, -0.5),
                    Number.Mod(-2.5, -0.5)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q665", () =>
            let r = try {
                    Number.Mod(1, 5),
                    Number.Mod(-1, 5),
                    Number.Mod(1, -5),
                    Number.Mod(-1, -5),
                    Number.Mod(5, 7),
                    Number.Mod(-5, 7),
                    Number.Mod(5, -7),
                    Number.Mod(-5, -7)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q666", () =>
            let r = try {
                    try Number.Mod(1, 0) otherwise "err",
                    try Number.Mod(-1, 0) otherwise "err",
                    try Number.Mod(0, 0) otherwise "err",
                    try Number.Mod(1.5, 0) otherwise "err",
                    try Number.Mod(null, 5) otherwise "err",
                    try Number.Mod(5, null) otherwise "err",
                    try Number.Mod(null, null) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q667", () =>
            let r = try {
                    try Number.Mod(Number.NaN, 3) otherwise "err",
                    try Number.Mod(3, Number.NaN) otherwise "err",
                    try Number.Mod(Number.PositiveInfinity, 3) otherwise "err",
                    try Number.Mod(3, Number.PositiveInfinity) otherwise "err",
                    try Number.Mod(Number.NegativeInfinity, 3) otherwise "err",
                    try Number.Mod(3, Number.NegativeInfinity) otherwise "err",
                    try Number.Mod(Number.PositiveInfinity, Number.PositiveInfinity) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q668", () =>
            let r = try {
                    Number.Mod(10, 3),
                    Number.Mod(11, 3),
                    Number.Mod(12, 3),
                    Number.Mod(100, 7),
                    Number.Mod(-100, 7),
                    Number.Mod(100, -7),
                    Number.Mod(-100, -7),
                    Number.Mod(1000000, 17)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q669", () =>
            let r = try {
                    Number.Mod(0.1, 0.03),
                    Number.Mod(-0.1, 0.03),
                    Number.Mod(1.1, 1),
                    Number.Mod(-1.1, 1),
                    Number.Mod(1, 1.1),
                    Number.Mod(-1, 1.1),
                    Number.Mod(0.0001, 0.0001),
                    Number.Mod(0.0002, 0.0001)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q670-q676: Number.IntegerDivide overflow + sign matrix.

        SafeSerialize("q670", () =>
            let r = try {
                    Number.IntegerDivide(10, 3),
                    Number.IntegerDivide(-10, 3),
                    Number.IntegerDivide(10, -3),
                    Number.IntegerDivide(-10, -3),
                    Number.IntegerDivide(0, 5),
                    Number.IntegerDivide(0, -5),
                    Number.IntegerDivide(3, 3),
                    Number.IntegerDivide(-3, 3)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q671", () =>
            let r = try {
                    Number.IntegerDivide(7.5, 2),
                    Number.IntegerDivide(-7.5, 2),
                    Number.IntegerDivide(7.5, -2),
                    Number.IntegerDivide(-7.5, -2),
                    Number.IntegerDivide(7, 2.5),
                    Number.IntegerDivide(-7, 2.5),
                    Number.IntegerDivide(0.5, 0.25),
                    Number.IntegerDivide(-0.5, 0.25)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q672", () =>
            let r = try {
                    try Number.IntegerDivide(1, 0) otherwise "err",
                    try Number.IntegerDivide(-1, 0) otherwise "err",
                    try Number.IntegerDivide(0, 0) otherwise "err",
                    try Number.IntegerDivide(1.5, 0) otherwise "err",
                    try Number.IntegerDivide(null, 5) otherwise "err",
                    try Number.IntegerDivide(5, null) otherwise "err",
                    try Number.IntegerDivide(null, null) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q673", () =>
            let r = try {
                    try Number.IntegerDivide(Number.NaN, 3) otherwise "err",
                    try Number.IntegerDivide(3, Number.NaN) otherwise "err",
                    try Number.IntegerDivide(Number.PositiveInfinity, 3) otherwise "err",
                    try Number.IntegerDivide(3, Number.PositiveInfinity) otherwise "err",
                    try Number.IntegerDivide(Number.NegativeInfinity, 3) otherwise "err",
                    try Number.IntegerDivide(3, Number.NegativeInfinity) otherwise "err",
                    try Number.IntegerDivide(Number.PositiveInfinity, Number.NegativeInfinity) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q674", () =>
            // Edge values near i64 bounds.
            let r = try {
                    Number.IntegerDivide(9007199254740992, 2),
                    Number.IntegerDivide(-9007199254740992, 2),
                    Number.IntegerDivide(9007199254740992, -2),
                    Number.IntegerDivide(-9007199254740992, -2),
                    Number.IntegerDivide(9007199254740992, 9007199254740992),
                    Number.IntegerDivide(9223372036854775000, 1),
                    Number.IntegerDivide(-9223372036854775000, 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q675", () =>
            let r = try {
                    Number.IntegerDivide(100, -1),
                    Number.IntegerDivide(-100, -1),
                    Number.IntegerDivide(100, 1),
                    Number.IntegerDivide(-100, 1),
                    Number.IntegerDivide(1, 100),
                    Number.IntegerDivide(-1, 100),
                    Number.IntegerDivide(99, 100),
                    Number.IntegerDivide(-99, 100)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q676", () =>
            // IntegerDivide-Mod relation: (a - a mod b) / b == IntegerDivide(a, b).
            let r = try
                    let
                        a1 = 17, b1 = 5,
                        a2 = -17, b2 = 5,
                        a3 = 17, b3 = -5,
                        a4 = -17, b4 = -5
                    in {
                        Number.IntegerDivide(a1, b1) = (a1 - Number.Mod(a1, b1)) / b1,
                        Number.IntegerDivide(a2, b2) = (a2 - Number.Mod(a2, b2)) / b2,
                        Number.IntegerDivide(a3, b3) = (a3 - Number.Mod(a3, b3)) / b3,
                        Number.IntegerDivide(a4, b4) = (a4 - Number.Mod(a4, b4)) / b4
                    }
            in
                if r[HasError]
                    then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                    else [HasError=false, Value=r[Value]]),

        // q677-q683: Number.Power edge cases.

        SafeSerialize("q677", () =>
            let r = try {
                    Number.Power(2, 10),
                    Number.Power(10, 3),
                    Number.Power(3, 0),
                    Number.Power(2, -3),
                    Number.Power(1, 100),
                    Number.Power(1, -100),
                    Number.Power(0, 5),
                    Number.Power(0, 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q678", () =>
            let r = try {
                    try Number.Power(-2, 0.5) otherwise "err",
                    try Number.Power(-2, 1.5) otherwise "err",
                    try Number.Power(-1, 0.5) otherwise "err",
                    try Number.Power(-1, 0.3) otherwise "err",
                    Number.Power(-2, 2),
                    Number.Power(-2, 3),
                    Number.Power(-2, -2),
                    Number.Power(-2, -3)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q679", () =>
            let r = try {
                    Number.Power(0, 0),
                    try Number.Power(0, -1) otherwise "err",
                    try Number.Power(0, -0.5) otherwise "err",
                    try Number.Power(0, -2) otherwise "err",
                    Number.Power(0, 0.5),
                    Number.Power(0, 1.5),
                    Number.Power(-0, 0),
                    Number.Power(-0, 2)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q680", () =>
            let r = try {
                    try Number.Power(Number.PositiveInfinity, 0) otherwise "err",
                    try Number.Power(Number.PositiveInfinity, 1) otherwise "err",
                    try Number.Power(Number.PositiveInfinity, -1) otherwise "err",
                    try Number.Power(1, Number.PositiveInfinity) otherwise "err",
                    try Number.Power(1, Number.NaN) otherwise "err",
                    try Number.Power(Number.NaN, 0) otherwise "err",
                    try Number.Power(Number.NaN, Number.NaN) otherwise "err",
                    try Number.Power(Number.NegativeInfinity, 2) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q681", () =>
            let r = try {
                    Number.Power(2, 53),
                    Number.Power(2, 62),
                    Number.Power(0.5, 10),
                    Number.Power(0.5, 50),
                    Number.Power(10, 100),
                    Number.Power(10, -100),
                    Number.Power(1.0000001, 1000000),
                    Number.Power(0.9999999, 1000000)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q682", () =>
            let r = try {
                    Number.Power(2, 0.5),
                    Number.Power(4, 0.5),
                    Number.Power(8, 1/3),
                    Number.Power(27, 1/3),
                    Number.Power(2, 1/3),
                    Number.Power(100, 0.5),
                    Number.Power(0.25, 0.5),
                    Number.Power(1, 0.5)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q683", () =>
            let r = try {
                    try Number.Power(null, 5) otherwise "err",
                    try Number.Power(5, null) otherwise "err",
                    try Number.Power(null, null) otherwise "err",
                    try Number.Power(null, 0) otherwise "err",
                    Number.Power(2, 1024),
                    Number.Power(2, 1023),
                    Number.Power(2, -1024),
                    Number.Power(2, -1074)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q684-q690: Number.Sqrt/Log/Ln/Exp boundaries.

        SafeSerialize("q684", () =>
            let r = try {
                    Number.Sqrt(0),
                    Number.Sqrt(1),
                    Number.Sqrt(4),
                    Number.Sqrt(2),
                    Number.Sqrt(0.25),
                    Number.Sqrt(1e100),
                    Number.Sqrt(1e-100),
                    try Number.Sqrt(-1) otherwise "err",
                    try Number.Sqrt(-0.0001) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q685", () =>
            let r = try {
                    try Number.Sqrt(Number.PositiveInfinity) otherwise "err",
                    try Number.Sqrt(Number.NegativeInfinity) otherwise "err",
                    try Number.Sqrt(Number.NaN) otherwise "err",
                    try Number.Sqrt(null) otherwise "err",
                    try Number.Sqrt(-0.0) otherwise "err",
                    Number.Sqrt(0.0)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q686", () =>
            let r = try {
                    Number.Ln(1),
                    try Number.Ln(0) otherwise "err",
                    try Number.Ln(-1) otherwise "err",
                    try Number.Ln(-0.0001) otherwise "err",
                    Number.Ln(2.718281828459045),
                    Number.Ln(10),
                    Number.Ln(0.5),
                    Number.Ln(1e100),
                    Number.Ln(1e-100)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q687", () =>
            let r = try {
                    try Number.Ln(Number.PositiveInfinity) otherwise "err",
                    try Number.Ln(Number.NegativeInfinity) otherwise "err",
                    try Number.Ln(Number.NaN) otherwise "err",
                    try Number.Ln(null) otherwise "err",
                    Number.Log10(1),
                    Number.Log10(10),
                    Number.Log10(100),
                    Number.Log10(0.1),
                    try Number.Log10(0) otherwise "err",
                    try Number.Log10(-1) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q688", () =>
            let r = try {
                    Number.Log(8, 2),
                    Number.Log(100, 10),
                    Number.Log(1, 10),
                    Number.Log(2, 2),
                    Number.Log(0.5, 2),
                    try Number.Log(0, 2) otherwise "err",
                    try Number.Log(-1, 2) otherwise "err",
                    try Number.Log(8, 1) otherwise "err",
                    try Number.Log(8, 0) otherwise "err",
                    try Number.Log(8, -2) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q689", () =>
            let r = try {
                    Number.Exp(0),
                    Number.Exp(1),
                    Number.Exp(-1),
                    Number.Exp(10),
                    Number.Exp(-10),
                    Number.Exp(100),
                    Number.Exp(-100),
                    Number.Exp(1000),
                    Number.Exp(-1000)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q690", () =>
            let r = try {
                    try Number.Exp(Number.PositiveInfinity) otherwise "err",
                    try Number.Exp(Number.NegativeInfinity) otherwise "err",
                    try Number.Exp(Number.NaN) otherwise "err",
                    try Number.Exp(null) otherwise "err",
                    Number.Ln(Number.Exp(2)),
                    Number.Exp(Number.Ln(2)),
                    Number.Sqrt(Number.Power(2, 4)),
                    Number.Power(Number.Sqrt(2), 2)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q691-q697: Number.ToText "F" precision.

        SafeSerialize("q691", () =>
            let r = try {
                    Number.ToText(3.14159, "F0"),
                    Number.ToText(3.14159, "F1"),
                    Number.ToText(3.14159, "F2"),
                    Number.ToText(0, "F2"),
                    Number.ToText(-3.14159, "F2"),
                    Number.ToText(0.5, "F0"),
                    Number.ToText(1.5, "F0"),
                    Number.ToText(2.5, "F0")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q692", () =>
            let r = try {
                    Number.ToText(3.14159265358979, "F5"),
                    Number.ToText(3.14159265358979, "F10"),
                    Number.ToText(0, "F5"),
                    Number.ToText(-3.14159265358979, "F5"),
                    Number.ToText(1, "F5"),
                    Number.ToText(1.23456789, "F5"),
                    Number.ToText(1.23456789, "F10")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q693", () =>
            let r = try {
                    Number.ToText(0.1, "F20"),
                    Number.ToText(0.2, "F20"),
                    Number.ToText(0.3, "F20"),
                    Number.ToText(1, "F20"),
                    Number.ToText(0, "F20"),
                    Number.ToText(-0.1, "F20")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q694", () =>
            let r = try {
                    Number.ToText(0.5, "F0"),
                    Number.ToText(1.5, "F0"),
                    Number.ToText(2.5, "F0"),
                    Number.ToText(3.5, "F0"),
                    Number.ToText(-0.5, "F0"),
                    Number.ToText(-1.5, "F0"),
                    Number.ToText(0.05, "F1"),
                    Number.ToText(0.15, "F1"),
                    Number.ToText(0.25, "F1"),
                    Number.ToText(0.35, "F1")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q695", () =>
            let r = try {
                    Number.ToText(1234567.89, "F2"),
                    Number.ToText(0.0001, "F4"),
                    Number.ToText(0.0001, "F2"),
                    Number.ToText(1e10, "F2"),
                    Number.ToText(1e-10, "F12"),
                    Number.ToText(123456789012345, "F0"),
                    Number.ToText(-1234567.89, "F2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q696", () =>
            let r = try {
                    try Number.ToText(Number.NaN, "F2") otherwise "err",
                    try Number.ToText(Number.PositiveInfinity, "F2") otherwise "err",
                    try Number.ToText(Number.NegativeInfinity, "F2") otherwise "err",
                    try Number.ToText(null, "F2") otherwise "err",
                    Number.ToText(3.14159, "F"),
                    Number.ToText(3.14159, "f0"),
                    Number.ToText(3.14159, "f2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q697", () =>
            let r = try {
                    Number.ToText(3.14, "F2", "en-US"),
                    Number.ToText(3.14, "F2", "en-GB"),
                    Number.ToText(3.14, "F2", "de-DE"),
                    Number.ToText(3.14, "F2", "fr-FR"),
                    Number.ToText(1234.5, "F2", "en-US"),
                    Number.ToText(1234.5, "F2", "de-DE"),
                    Number.ToText(-0.5, "F1", "de-DE")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q698-q704: Number.ToText "N" precision + culture.

        SafeSerialize("q698", () =>
            let r = try {
                    Number.ToText(1234567, "N0"),
                    Number.ToText(1234567, "N2"),
                    Number.ToText(1234567.89, "N0"),
                    Number.ToText(1234567.89, "N2"),
                    Number.ToText(0, "N0"),
                    Number.ToText(0, "N2"),
                    Number.ToText(-1234567.89, "N2"),
                    Number.ToText(999, "N2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q699", () =>
            let r = try {
                    Number.ToText(1234567.123456, "N1"),
                    Number.ToText(1234567.123456, "N3"),
                    Number.ToText(1234567.123456, "N5"),
                    Number.ToText(1234567.123456, "N10"),
                    Number.ToText(0.5, "N2"),
                    Number.ToText(0.05, "N2"),
                    Number.ToText(0.005, "N2"),
                    Number.ToText(0.0005, "N2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q700", () =>
            let r = try {
                    Number.ToText(1234567.89, "N2", "en-US"),
                    Number.ToText(1234567.89, "N2", "en-GB"),
                    Number.ToText(1234567.89, "N2", "de-DE"),
                    Number.ToText(1234567.89, "N2", "fr-FR"),
                    Number.ToText(-1234567.89, "N2", "de-DE"),
                    Number.ToText(0, "N2", "de-DE"),
                    Number.ToText(0.5, "N2", "fr-FR")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q701", () =>
            let r = try {
                    Number.ToText(0.5, "N0"),
                    Number.ToText(1.5, "N0"),
                    Number.ToText(2.5, "N0"),
                    Number.ToText(-0.5, "N0"),
                    Number.ToText(-1.5, "N0"),
                    Number.ToText(0.005, "N2"),
                    Number.ToText(0.015, "N2"),
                    Number.ToText(0.025, "N2"),
                    Number.ToText(0.035, "N2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q702", () =>
            let r = try {
                    Number.ToText(1e15, "N0"),
                    Number.ToText(1e15, "N2"),
                    Number.ToText(1e-5, "N6"),
                    Number.ToText(1e-10, "N12"),
                    Number.ToText(123456789012345, "N0"),
                    Number.ToText(-123456789012345, "N0"),
                    Number.ToText(1e10, "N0", "de-DE"),
                    Number.ToText(1e10, "N0", "fr-FR")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q703", () =>
            let r = try {
                    try Number.ToText(Number.NaN, "N2") otherwise "err",
                    try Number.ToText(Number.PositiveInfinity, "N2") otherwise "err",
                    try Number.ToText(Number.NegativeInfinity, "N2") otherwise "err",
                    try Number.ToText(null, "N2") otherwise "err",
                    Number.ToText(1234.5, "N"),
                    Number.ToText(1234.5, "n2"),
                    Number.ToText(1234.5, "n0")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q704", () =>
            let r = try {
                    Number.ToText(999, "N0"),
                    Number.ToText(1000, "N0"),
                    Number.ToText(9999, "N0"),
                    Number.ToText(10000, "N0"),
                    Number.ToText(100000, "N0"),
                    Number.ToText(999.99, "N2"),
                    Number.ToText(1000.99, "N2"),
                    Number.ToText(-999.99, "N2"),
                    Number.ToText(-1000.99, "N2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q705-q711: Number.ToText "P" percentage.

        SafeSerialize("q705", () =>
            let r = try {
                    Number.ToText(0.5, "P0"),
                    Number.ToText(0.5, "P1"),
                    Number.ToText(0.5, "P2"),
                    Number.ToText(0.5, "P5"),
                    Number.ToText(0, "P2"),
                    Number.ToText(1, "P0"),
                    Number.ToText(1, "P2"),
                    Number.ToText(-0.5, "P2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q706", () =>
            let r = try {
                    Number.ToText(0.125, "P0"),
                    Number.ToText(0.125, "P1"),
                    Number.ToText(0.125, "P2"),
                    Number.ToText(0.001, "P0"),
                    Number.ToText(0.001, "P2"),
                    Number.ToText(0.0001, "P2"),
                    Number.ToText(0.0001, "P4"),
                    Number.ToText(0.9999, "P0"),
                    Number.ToText(0.9999, "P2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q707", () =>
            let r = try {
                    Number.ToText(0.5, "P2", "en-US"),
                    Number.ToText(0.5, "P2", "en-GB"),
                    Number.ToText(0.5, "P2", "de-DE"),
                    Number.ToText(0.5, "P2", "fr-FR"),
                    Number.ToText(0.123, "P2", "en-US"),
                    Number.ToText(0.123, "P2", "de-DE"),
                    Number.ToText(-0.123, "P2", "fr-FR"),
                    Number.ToText(1234.5, "P2", "en-US")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q708", () =>
            let r = try {
                    Number.ToText(0.005, "P0"),
                    Number.ToText(0.015, "P0"),
                    Number.ToText(0.025, "P0"),
                    Number.ToText(0.005, "P1"),
                    Number.ToText(0.015, "P1"),
                    Number.ToText(0.025, "P1"),
                    Number.ToText(-0.005, "P0"),
                    Number.ToText(-0.015, "P0")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q709", () =>
            let r = try {
                    Number.ToText(1e-10, "P10"),
                    Number.ToText(1e-15, "P15"),
                    Number.ToText(100, "P0"),
                    Number.ToText(1000, "P0"),
                    Number.ToText(0.000001, "P4"),
                    Number.ToText(0.000001, "P8"),
                    Number.ToText(1e10, "P0"),
                    Number.ToText(-1e6, "P2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q710", () =>
            let r = try {
                    try Number.ToText(Number.NaN, "P2") otherwise "err",
                    try Number.ToText(Number.PositiveInfinity, "P2") otherwise "err",
                    try Number.ToText(Number.NegativeInfinity, "P2") otherwise "err",
                    try Number.ToText(null, "P2") otherwise "err",
                    Number.ToText(0.5, "P"),
                    Number.ToText(0.5, "p0"),
                    Number.ToText(0.5, "p2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q711", () =>
            let r = try {
                    Number.ToText(12.345, "P2"),
                    Number.ToText(12345.6789, "P0"),
                    Number.ToText(12345.6789, "P2"),
                    Number.ToText(12345.6789, "P2", "de-DE"),
                    Number.ToText(12345.6789, "P2", "fr-FR"),
                    Number.ToText(-12345.6789, "P2"),
                    Number.ToText(-12345.6789, "P2", "de-DE")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q712-q718: Number.ToText "C" currency.

        SafeSerialize("q712", () =>
            let r = try {
                    Number.ToText(1234.5, "C"),
                    Number.ToText(1234.5, "C0"),
                    Number.ToText(1234.5, "C2"),
                    Number.ToText(1234.5, "C4"),
                    Number.ToText(0, "C2"),
                    Number.ToText(0.5, "C2"),
                    Number.ToText(-1234.5, "C2"),
                    Number.ToText(-0.5, "C2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q713", () =>
            let r = try {
                    Number.ToText(1234.5, "C2", "en-US"),
                    Number.ToText(1234.5, "C2", "en-GB"),
                    Number.ToText(1234.5, "C0", "ja-JP"),
                    Number.ToText(1234.5, "C2", "de-DE"),
                    Number.ToText(1234.5, "C2", "fr-FR"),
                    Number.ToText(-1234.5, "C2", "en-US"),
                    Number.ToText(-1234.5, "C2", "de-DE"),
                    Number.ToText(-1234.5, "C2", "fr-FR")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q714", () =>
            let r = try {
                    Number.ToText(1234567.89, "C0", "en-US"),
                    Number.ToText(1234567.89, "C2", "en-US"),
                    Number.ToText(1234567.89, "C4", "en-US"),
                    Number.ToText(1234567.89, "C0", "en-GB"),
                    Number.ToText(1234567.89, "C2", "en-GB"),
                    Number.ToText(1234567.89, "C0", "ja-JP"),
                    Number.ToText(1234567.89, "C2", "de-DE"),
                    Number.ToText(1234567.89, "C2", "fr-FR")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q715", () =>
            let r = try {
                    Number.ToText(0.5, "C0"),
                    Number.ToText(1.5, "C0"),
                    Number.ToText(2.5, "C0"),
                    Number.ToText(-0.5, "C0"),
                    Number.ToText(-1.5, "C0"),
                    Number.ToText(0.005, "C2"),
                    Number.ToText(0.015, "C2"),
                    Number.ToText(0.025, "C2"),
                    Number.ToText(-0.005, "C2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q716", () =>
            let r = try {
                    try Number.ToText(Number.NaN, "C2") otherwise "err",
                    try Number.ToText(Number.PositiveInfinity, "C2") otherwise "err",
                    try Number.ToText(Number.NegativeInfinity, "C2") otherwise "err",
                    try Number.ToText(null, "C2") otherwise "err",
                    Number.ToText(1234.5, "C"),
                    Number.ToText(1234.5, "c0"),
                    Number.ToText(1234.5, "c2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q717", () =>
            let r = try {
                    Number.ToText(0, "C", "en-US"),
                    Number.ToText(0.01, "C2", "en-US"),
                    Number.ToText(1e-5, "C6", "en-US"),
                    Number.ToText(1e10, "C0", "en-US"),
                    Number.ToText(1e15, "C0", "en-US"),
                    Number.ToText(123456789012345, "C0", "en-US"),
                    Number.ToText(-123456789012345, "C0", "en-US")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q718", () =>
            let r = try {
                    Number.ToText(-100, "C2", "en-US"),
                    Number.ToText(-100, "C2", "en-GB"),
                    Number.ToText(-100, "C0", "ja-JP"),
                    Number.ToText(-100, "C2", "de-DE"),
                    Number.ToText(-100, "C2", "fr-FR"),
                    Number.ToText(0, "C2", "en-US"),
                    Number.ToText(0, "C2", "de-DE"),
                    Number.ToText(0, "C2", "fr-FR")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q719-q725: Number.ToText "D" integer padding.

        SafeSerialize("q719", () =>
            let r = try {
                    Number.ToText(42, "D"),
                    Number.ToText(42, "D0"),
                    Number.ToText(42, "D1"),
                    Number.ToText(42, "D5"),
                    Number.ToText(42, "D10"),
                    Number.ToText(42, "D20"),
                    Number.ToText(0, "D5"),
                    Number.ToText(7, "D3")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q720", () =>
            let r = try {
                    Number.ToText(-42, "D"),
                    Number.ToText(-42, "D0"),
                    Number.ToText(-42, "D5"),
                    Number.ToText(-42, "D10"),
                    Number.ToText(-7, "D3"),
                    Number.ToText(-0, "D5"),
                    Number.ToText(-1, "D5"),
                    Number.ToText(-99999, "D5")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q721", () =>
            let r = try {
                    Number.ToText(2147483647, "D"),
                    Number.ToText(2147483647, "D15"),
                    Number.ToText(-2147483648, "D15"),
                    Number.ToText(9223372036854775000, "D"),
                    Number.ToText(-9223372036854775000, "D"),
                    Number.ToText(0, "D0"),
                    Number.ToText(0, "D10"),
                    Number.ToText(1000000, "D0"),
                    Number.ToText(1000000, "D10")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q722", () =>
            let r = try {
                    try Number.ToText(3.7, "D") otherwise "err",
                    try Number.ToText(3.7, "D5") otherwise "err",
                    try Number.ToText(-3.7, "D5") otherwise "err",
                    try Number.ToText(0.5, "D5") otherwise "err",
                    try Number.ToText(-0.5, "D5") otherwise "err",
                    try Number.ToText(3.0, "D5") otherwise "err",
                    try Number.ToText(0.0, "D5") otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q723", () =>
            let r = try {
                    try Number.ToText(Number.NaN, "D5") otherwise "err",
                    try Number.ToText(Number.PositiveInfinity, "D5") otherwise "err",
                    try Number.ToText(Number.NegativeInfinity, "D5") otherwise "err",
                    try Number.ToText(null, "D5") otherwise "err",
                    Number.ToText(42, "d"),
                    Number.ToText(42, "d3")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q724", () =>
            let r = try {
                    Number.ToText(42, "D5", "en-US"),
                    Number.ToText(42, "D5", "en-GB"),
                    Number.ToText(42, "D5", "de-DE"),
                    Number.ToText(42, "D5", "fr-FR"),
                    Number.ToText(42, "D5", "ja-JP"),
                    Number.ToText(-42, "D5", "de-DE"),
                    Number.ToText(0, "D5", "fr-FR")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q725", () =>
            let r = try {
                    Number.ToText(99999, "D5"),
                    Number.ToText(100000, "D5"),
                    Number.ToText(100000, "D6"),
                    Number.ToText(123456789, "D5"),
                    Number.ToText(123456789, "D20"),
                    Number.ToText(-99999, "D5"),
                    Number.ToText(-100000, "D5"),
                    Number.ToText(-100000, "D6")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q726-q732: Number.ToText "E" exponent format.

        SafeSerialize("q726", () =>
            let r = try {
                    Number.ToText(1234.5, "E"),
                    Number.ToText(1234.5, "E0"),
                    Number.ToText(1234.5, "E2"),
                    Number.ToText(1234.5, "E6"),
                    Number.ToText(0, "E2"),
                    Number.ToText(-1234.5, "E2"),
                    Number.ToText(0.000123, "E2"),
                    Number.ToText(-0.000123, "E2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q727", () =>
            let r = try {
                    Number.ToText(3.14159265358979, "E0"),
                    Number.ToText(3.14159265358979, "E1"),
                    Number.ToText(3.14159265358979, "E5"),
                    Number.ToText(3.14159265358979, "E10"),
                    Number.ToText(3.14159265358979, "E15"),
                    Number.ToText(3.14159265358979, "E20"),
                    Number.ToText(1, "E5"),
                    Number.ToText(0.5, "E5")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q728", () =>
            let r = try {
                    Number.ToText(1234.5, "E2"),
                    Number.ToText(1234.5, "e2"),
                    Number.ToText(1234.5, "E2", "en-US"),
                    Number.ToText(1234.5, "E2", "de-DE"),
                    Number.ToText(1234.5, "E2", "fr-FR"),
                    Number.ToText(-1234.5, "e2", "de-DE"),
                    Number.ToText(0.0001, "e3", "fr-FR")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q729", () =>
            let r = try {
                    Number.ToText(1e100, "E2"),
                    Number.ToText(1e-100, "E2"),
                    Number.ToText(1e308, "E2"),
                    Number.ToText(1e-308, "E2"),
                    Number.ToText(2.225e-308, "E5"),
                    Number.ToText(1.7976931348623157e308, "E5"),
                    Number.ToText(-1e100, "E3"),
                    Number.ToText(-1e-100, "E3")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q730", () =>
            let r = try {
                    try Number.ToText(Number.NaN, "E2") otherwise "err",
                    try Number.ToText(Number.PositiveInfinity, "E2") otherwise "err",
                    try Number.ToText(Number.NegativeInfinity, "E2") otherwise "err",
                    try Number.ToText(null, "E2") otherwise "err",
                    Number.ToText(0.0, "E2"),
                    Number.ToText(-0.0, "E2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q731", () =>
            let r = try {
                    Number.ToText(1.5, "E0"),
                    Number.ToText(2.5, "E0"),
                    Number.ToText(3.5, "E0"),
                    Number.ToText(-1.5, "E0"),
                    Number.ToText(-2.5, "E0"),
                    Number.ToText(1.25, "E1"),
                    Number.ToText(2.25, "E1"),
                    Number.ToText(0.0005, "E0"),
                    Number.ToText(0.0015, "E0")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q732", () =>
            let r = try {
                    Number.ToText(1, "E2"),
                    Number.ToText(10, "E2"),
                    Number.ToText(100, "E2"),
                    Number.ToText(1000, "E2"),
                    Number.ToText(1e10, "E2"),
                    Number.ToText(0.1, "E2"),
                    Number.ToText(0.01, "E2"),
                    Number.ToText(0.001, "E2")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q733-q739: Number.ToText custom patterns.

        SafeSerialize("q733", () =>
            let r = try {
                    Number.ToText(3.14159, "0.00"),
                    Number.ToText(3.14159, "0.000"),
                    Number.ToText(3.14159, "0.00000"),
                    Number.ToText(0, "0.00"),
                    Number.ToText(0.5, "0.00"),
                    Number.ToText(-3.14159, "0.00"),
                    Number.ToText(1234, "0.00"),
                    Number.ToText(1234.5, "0")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q734", () =>
            let r = try {
                    Number.ToText(3.14, "#.##"),
                    Number.ToText(3.1, "#.##"),
                    Number.ToText(3, "#.##"),
                    Number.ToText(3.14, "#.00"),
                    Number.ToText(3.1, "#.00"),
                    Number.ToText(3, "#.00"),
                    Number.ToText(0.5, "#.##"),
                    Number.ToText(0.5, "0.##")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q735", () =>
            let r = try {
                    Number.ToText(1234.5, "#,##0.00"),
                    Number.ToText(1234567.89, "#,##0.00"),
                    Number.ToText(0, "#,##0.00"),
                    Number.ToText(-1234.5, "#,##0.00"),
                    Number.ToText(1234567, "#,##0"),
                    Number.ToText(999, "#,##0"),
                    Number.ToText(1234.5, "#,##0.00", "de-DE"),
                    Number.ToText(1234.5, "#,##0.00", "fr-FR")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q736", () =>
            let r = try {
                    Number.ToText(0.5, "0.00%"),
                    Number.ToText(0.5, "0%"),
                    Number.ToText(0.123, "0.00%"),
                    Number.ToText(0.123, "0.0%"),
                    Number.ToText(0, "0.00%"),
                    Number.ToText(-0.5, "0.00%"),
                    Number.ToText(1, "0.00%")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q737", () =>
            let r = try {
                    Number.ToText(1234.5, "#.##E+0"),
                    Number.ToText(1234.5, "0.00E+00"),
                    Number.ToText(1234.5, "0.00E+000"),
                    Number.ToText(0.001234, "0.00E+00"),
                    Number.ToText(-1234.5, "0.00E+00"),
                    Number.ToText(0, "0.00E+00"),
                    Number.ToText(1, "0.00E+00")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q738", () =>
            let r = try {
                    Number.ToText(42, "00000"),
                    Number.ToText(42, "000"),
                    Number.ToText(42, "0"),
                    Number.ToText(0, "00000"),
                    Number.ToText(-42, "00000"),
                    Number.ToText(123456, "00000"),
                    Number.ToText(1.5, "00000"),
                    Number.ToText(0.5, "0000")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q739", () =>
            let r = try {
                    Number.ToText(1.5, "0.00;-0.00;zero"),
                    Number.ToText(-1.5, "0.00;-0.00;zero"),
                    Number.ToText(0, "0.00;-0.00;zero"),
                    Number.ToText(1.5, "0.00;(0.00)"),
                    Number.ToText(-1.5, "0.00;(0.00)"),
                    Number.ToText(0, "0.00;(0.00)"),
                    try Number.ToText(Number.NaN, "0.00") otherwise "err",
                    try Number.ToText(Number.PositiveInfinity, "0.00") otherwise "err",
                    try Number.ToText(null, "0.00") otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q740-q746: Number.FromText edge inputs.

        SafeSerialize("q740", () =>
            let r = try {
                    Number.FromText("42"),
                    Number.FromText(" 42"),
                    Number.FromText("42 "),
                    Number.FromText("  42  "),
                    Number.FromText("#(tab)42#(tab)"),
                    Number.FromText("3.14"),
                    Number.FromText(" 3.14 "),
                    try Number.FromText("") otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q741", () =>
            let r = try {
                    Number.FromText("-42"),
                    Number.FromText("+42"),
                    Number.FromText("-3.14"),
                    Number.FromText("1e5"),
                    Number.FromText("1E5"),
                    Number.FromText("1e+05"),
                    Number.FromText("1e-5"),
                    Number.FromText("-1.5e10"),
                    Number.FromText("1.5E-10")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q742", () =>
            let r = try {
                    try Number.FromText("$100") otherwise "err",
                    try Number.FromText("$100.50") otherwise "err",
                    try Number.FromText("£100") otherwise "err",
                    try Number.FromText("€100") otherwise "err",
                    try Number.FromText("(100)") otherwise "err",
                    try Number.FromText("(100.50)") otherwise "err",
                    try Number.FromText("($100)") otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q743", () =>
            let r = try {
                    Number.FromText("1,000"),
                    Number.FromText("1,234,567"),
                    Number.FromText("1,234.5"),
                    Number.FromText("-1,234.5"),
                    try Number.FromText("1.234.567,89") otherwise "err",
                    Number.FromText("1.234.567,89", "de-DE"),
                    Number.FromText("1234,5", "de-DE"),
                    Number.FromText("1 234 567,89", "fr-FR")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q744", () =>
            let r = try {
                    try Number.FromText("abc") otherwise "err",
                    try Number.FromText("12abc") otherwise "err",
                    try Number.FromText("1..2") otherwise "err",
                    try Number.FromText("1.2.3") otherwise "err",
                    try Number.FromText("--1") otherwise "err",
                    try Number.FromText("+-1") otherwise "err",
                    try Number.FromText("1e") otherwise "err",
                    try Number.FromText("e5") otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q745", () =>
            let r = try {
                    try Number.FromText("Infinity") otherwise "err",
                    try Number.FromText("-Infinity") otherwise "err",
                    try Number.FromText("NaN") otherwise "err",
                    try Number.FromText("inf") otherwise "err",
                    Number.FromText("0"),
                    Number.FromText("0.0"),
                    Number.FromText("-0"),
                    Number.FromText("-0.0"),
                    try Number.FromText("null") otherwise "err",
                    try Number.FromText(null) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q746", () =>
            let r = try {
                    Number.FromText("1e100"),
                    Number.FromText("1e-100"),
                    Number.FromText("1e308"),
                    Number.FromText("1e-308"),
                    Number.FromText("1.7976931348623157e308"),
                    Number.FromText("9007199254740992"),
                    Number.FromText("9223372036854775807"),
                    Number.FromText("0.000000000000000000001"),
                    Number.FromText("1234567890123456")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q747-q753: Number.Sign / Number.Abs edge cases.

        SafeSerialize("q747", () =>
            let r = try {
                    Number.Sign(5),
                    Number.Sign(-5),
                    Number.Sign(0),
                    Number.Sign(0.0),
                    Number.Sign(-0.0),
                    Number.Sign(0.001),
                    Number.Sign(-0.001),
                    Number.Sign(1e-300),
                    Number.Sign(-1e-300)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q748", () =>
            let r = try {
                    try Number.Sign(Number.NaN) otherwise "err",
                    try Number.Sign(Number.PositiveInfinity) otherwise "err",
                    try Number.Sign(Number.NegativeInfinity) otherwise "err",
                    try Number.Sign(null) otherwise "err",
                    Number.Sign(1234567890),
                    Number.Sign(-1234567890),
                    Number.Sign(1e308),
                    Number.Sign(-1e308)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q749", () =>
            let r = try {
                    Number.Abs(5),
                    Number.Abs(-5),
                    Number.Abs(0),
                    Number.Abs(0.0),
                    Number.Abs(-0.0),
                    Number.Abs(3.14),
                    Number.Abs(-3.14),
                    Number.Abs(1e-300),
                    Number.Abs(-1e-300)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q750", () =>
            let r = try {
                    try Number.Abs(Number.NaN) otherwise "err",
                    try Number.Abs(Number.PositiveInfinity) otherwise "err",
                    try Number.Abs(Number.NegativeInfinity) otherwise "err",
                    try Number.Abs(null) otherwise "err",
                    Number.Abs(1e308),
                    Number.Abs(-1e308),
                    Number.Abs(-1.7976931348623157e308),
                    Number.Abs(-9007199254740992)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q751", () =>
            let r = try {
                    Number.Sign(42) * Number.Abs(42),
                    Number.Sign(-42) * Number.Abs(-42),
                    Number.Sign(3.14) * Number.Abs(3.14),
                    Number.Sign(-3.14) * Number.Abs(-3.14),
                    Number.Sign(0) * Number.Abs(0),
                    Number.Sign(1e10) * Number.Abs(1e10),
                    Number.Sign(-1e-10) * Number.Abs(-1e-10)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q752", () =>
            let r = try {
                    Number.Abs(4.9e-324),
                    Number.Abs(-4.9e-324),
                    Number.Sign(4.9e-324),
                    Number.Sign(-4.9e-324),
                    Number.Abs(2.2250738585072014e-308),
                    Number.Sign(2.2250738585072014e-308),
                    Number.Sign(1e-323),
                    Number.Sign(-1e-323)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q753", () =>
            let r = try {
                    Number.Sign(Number.Sqrt(-1)),
                    Number.Abs(Number.Sqrt(-1)),
                    try Number.Sign(Number.Power(-2, 0.5)) otherwise "err",
                    try Number.Abs(Number.Power(-2, 0.5)) otherwise "err",
                    Number.Sign(Number.Round(0.001, 0)),
                    Number.Abs(Number.Round(-0.001, 0)),
                    Number.Sign(Number.Mod(-5, 3)),
                    Number.Abs(Number.Mod(-5, 3))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        // q754-q760: Bitwise ops on signed / unsigned bounds.

        SafeSerialize("q754", () =>
            let r = try {
                    Number.BitwiseAnd(0xFF, 0x0F),
                    Number.BitwiseAnd(0xFFFF, 0xFF00),
                    Number.BitwiseAnd(0, 0xFFFFFFFF),
                    Number.BitwiseAnd(-1, -1),
                    Number.BitwiseAnd(-1, 0),
                    Number.BitwiseAnd(-1, 1),
                    Number.BitwiseAnd(0x80000000, 0xFFFFFFFF),
                    Number.BitwiseAnd(0xFFFFFFFF, 0xFFFFFFFF)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q755", () =>
            let r = try {
                    Number.BitwiseOr(0xF0, 0x0F),
                    Number.BitwiseOr(-1, 0),
                    Number.BitwiseOr(0, -1),
                    Number.BitwiseXor(0xFF, 0xFF),
                    Number.BitwiseXor(0xFF, 0x00),
                    Number.BitwiseXor(-1, -1),
                    Number.BitwiseXor(-1, 0),
                    Number.BitwiseNot(0),
                    Number.BitwiseNot(-1),
                    Number.BitwiseNot(0xFF)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q756", () =>
            let r = try {
                    Number.BitwiseShiftLeft(1, 0),
                    Number.BitwiseShiftLeft(1, 1),
                    Number.BitwiseShiftLeft(1, 8),
                    Number.BitwiseShiftLeft(1, 30),
                    Number.BitwiseShiftLeft(1, 31),
                    Number.BitwiseShiftLeft(1, 62),
                    Number.BitwiseShiftLeft(1, 63),
                    Number.BitwiseShiftRight(256, 8),
                    Number.BitwiseShiftRight(0xFFFFFFFF, 16),
                    Number.BitwiseShiftRight(-1, 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q757", () =>
            let r = try {
                    try Number.BitwiseShiftLeft(1, 64) otherwise "err",
                    try Number.BitwiseShiftLeft(1, 65) otherwise "err",
                    try Number.BitwiseShiftLeft(1, 100) otherwise "err",
                    try Number.BitwiseShiftRight(1, 64) otherwise "err",
                    try Number.BitwiseShiftLeft(1, -1) otherwise "err",
                    try Number.BitwiseShiftRight(1, -1) otherwise "err",
                    try Number.BitwiseShiftLeft(1, 0.5) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q758", () =>
            // Stay within f64-exact-integer range (≤ 2^53) since mrsflow's
            // numeric literals go through f64. PQ uses Decimal internally
            // so it preserves 19-digit i64 literals exactly; this probe
            // sidesteps that parser-precision divergence.
            let r = try {
                    Number.BitwiseAnd(9007199254740991, 1),
                    Number.BitwiseAnd(9007199254740991, 0xFF),
                    Number.BitwiseOr(9007199254740990, 1),
                    Number.BitwiseXor(9007199254740991, 9007199254740991),
                    Number.BitwiseAnd(0x1FFFFFFFFFFFFF, 0xFFFFFFFF),
                    Number.BitwiseOr(0x100000000, 0xFFFFFFFF),
                    Number.BitwiseShiftRight(0x1FFFFFFFFFFFFF, 32),
                    Number.BitwiseShiftRight(0x1FFFFFFFFFFFFF, 52)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q759", () =>
            let r = try {
                    try Number.BitwiseAnd(3.5, 5) otherwise "err",
                    try Number.BitwiseAnd(5, 3.5) otherwise "err",
                    try Number.BitwiseAnd(Number.NaN, 5) otherwise "err",
                    try Number.BitwiseAnd(Number.PositiveInfinity, 5) otherwise "err",
                    try Number.BitwiseAnd(null, 5) otherwise "err",
                    try Number.BitwiseAnd(5, null) otherwise "err",
                    try Number.BitwiseShiftLeft(1.5, 2) otherwise "err",
                    try Number.BitwiseNot(3.5) otherwise "err",
                    try Number.BitwiseNot(null) otherwise "err"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),

        SafeSerialize("q760", () =>
            let r = try {
                    Number.BitwiseAnd(255, Number.BitwiseNot(0)) = 255,
                    Number.BitwiseXor(123, 123) = 0,
                    Number.BitwiseOr(123, 0) = 123,
                    Number.BitwiseAnd(123, 0) = 0,
                    Number.BitwiseShiftLeft(Number.BitwiseShiftRight(0xFF00, 8), 8) = 0xFF00,
                    Number.BitwiseXor(Number.BitwiseAnd(0xF0, 0xCC), Number.BitwiseOr(0xF0, 0xCC)) = Number.BitwiseXor(0xF0, 0xCC),
                    Number.BitwiseAnd(5, 3) + Number.BitwiseOr(5, 3) = 5 + 3,
                    Number.BitwiseAnd(0xAAAA, 0xFFFF) = 0xAAAA
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q761", () =>
            let r = try {
                    Text.Replace("Hello hello HELLO", "hello", "X", Comparer.OrdinalIgnoreCase),
                    Text.Replace("Hello", "HELLO", "X", Comparer.OrdinalIgnoreCase),
                    Text.Replace("Hello", "hello", "X", Comparer.Ordinal),
                    Text.Replace("Hello", "HELLO", "X", Comparer.Ordinal)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q762", () =>
            let r = try {
                    Text.Replace("abc", "", "X"),
                    Text.Replace("", "", "X"),
                    Text.Replace("abc", "", ""),
                    Text.Replace("", "abc", "X")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q763", () =>
            let r = try {
                    Text.Replace("abcabc", "b", ""),
                    Text.Replace("aaa", "a", ""),
                    Text.Replace("abc", "abc", ""),
                    Text.Replace("a-b-c", "-", "")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q764", () =>
            let r = try {
                    Text.Replace("aaaa", "aa", "b"),
                    Text.Replace("ababab", "ab", "X"),
                    Text.Replace("aaa", "aa", "a"),
                    Text.Replace("xx", "x", "xx")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q765", () =>
            let r = try {
                    Text.Replace("café", "é", "e"),
                    Text.Replace("naïve", "ï", "i"),
                    Text.Replace("→→→", "→", "->"),
                    Text.Replace("ßß", "ß", "ss")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q766", () =>
            let r = try {
                    Text.Replace(null, "a", "b"),
                    Text.Replace("abc", null, "b"),
                    Text.Replace("abc", "a", null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q767", () =>
            let r = try {
                    Text.Replace("Hello hello", "hello", "X", (a, b) => Value.Compare(Text.Lower(a), Text.Lower(b)))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q768", () =>
            let r = try {
                    Text.Split("abc", ""),
                    Text.Split("", ""),
                    Text.Split("a", ""),
                    Text.Split("→é!", "")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q769", () =>
            let r = try {
                    Text.Split("a--b--c", "--"),
                    Text.Split("a--", "--"),
                    Text.Split("--a", "--"),
                    Text.Split("----", "--"),
                    Text.Split("abc", "xyz")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q770", () =>
            let r = try {
                    Text.Split("a,,b", ","),
                    Text.Split(",a,b", ","),
                    Text.Split("a,b,", ","),
                    Text.Split(",,,", ","),
                    Text.Split(",", ",")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q771", () =>
            let r = try {
                    Text.SplitAny("a,b;c|d", ",;|"),
                    Text.SplitAny("a,,b", ",;"),
                    Text.SplitAny("abc", ""),
                    Text.SplitAny("", ",;"),
                    Text.SplitAny("→é→é", "→é")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q772", () =>
            let r = try {
                    Text.Split("abc", "abc"),
                    Text.Split("", "x"),
                    Text.Split("abc", "b"),
                    Text.Split("abcabc", "abc")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q773", () =>
            let r = try {
                    Text.SplitAny("a→b←c", "→←"),
                    Text.SplitAny("café", "é"),
                    Text.SplitAny("naïve", "ï"),
                    Text.SplitAny("abc", null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q774", () =>
            let r = try {
                    Text.Split(null, ","),
                    Text.Split("abc", null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q775", () =>
            let r = try {
                    Text.PadStart("abc", 0),
                    Text.PadStart("abc", 1),
                    Text.PadStart("abc", 2),
                    Text.PadStart("abc", 3),
                    Text.PadStart("", 0),
                    Text.PadStart("", 3)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q776", () =>
            let r = try {
                    Text.PadEnd("abc", 0),
                    Text.PadEnd("abc", 2),
                    Text.PadEnd("abc", 3),
                    Text.PadEnd("abc", 6, "X"),
                    Text.PadEnd("", 0),
                    Text.PadEnd("", 3, "*")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q777", () =>
            let r = try {
                    Text.PadStart("a", 5, "→"),
                    Text.PadStart("café", 6, "*"),
                    Text.PadStart("→", 3, "X"),
                    Text.PadStart("ß", 4, "→")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q778", () =>
            let r = try {
                    Text.PadStart("a", 5, "ab")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q779", () =>
            let r = try {
                    Text.PadEnd("a", 5, ""),
                    Text.PadEnd("a", -1),
                    Text.PadEnd("a", 1.5)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q780", () =>
            let r = try {
                    Text.PadStart(null, 5),
                    Text.PadStart("a", null),
                    Text.PadEnd(null, 5)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q781", () =>
            let r = try {
                    Text.PadStart("a", 5),
                    Text.PadEnd("a", 5),
                    Text.PadStart("", 3),
                    Text.PadEnd("", 3)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q782", () =>
            let r = try {
                    Text.Range("abc", 3),
                    Text.Range("abc", 3, 0),
                    Text.Range("abc", 4),
                    Text.Range("abc", 4, 0),
                    Text.Range("", 0),
                    Text.Range("", 0, 0)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q783", () =>
            let r = try {
                    Text.Range("abc", 0, 0),
                    Text.Range("abc", 1, 0),
                    Text.Range("abc", 0, 5),
                    Text.Range("abc", 1, 5),
                    Text.Range("abc", 0),
                    Text.Range("abc", 1),
                    Text.Range("abc", 2)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q784", () =>
            let r = try {
                    Text.Range("abc", -1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q785", () =>
            let r = try {
                    Text.Range("abc", 0, -1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q786", () =>
            let r = try {
                    Text.Range("café", 0, 3),
                    Text.Range("café", 1, 2),
                    Text.Range("café", 3, 1),
                    Text.Range("→→→", 0, 2),
                    Text.Range("naïve", 2, 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q787", () =>
            let r = try {
                    Text.Range(null, 0),
                    Text.Range("abc", null),
                    Text.Range("abc", 0, null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q788", () =>
            let r = try {
                    Text.Range("abc", 1.5),
                    Text.Range("abc", 0, 1.5)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q789", () =>
            let r = try {
                    Text.Range("abc", 0, 5)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q790", () =>
            let r = try {
                    Text.Range("abc", 3)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q791", () =>
            let r = try {
                    Text.Insert("abc", 0, "X"),
                    Text.Insert("abc", 1, "X"),
                    Text.Insert("abc", 2, "X"),
                    Text.Insert("abc", 3, "X")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q792", () =>
            let r = try {
                    Text.Insert("abc", 0, ""),
                    Text.Insert("abc", 1, ""),
                    Text.Insert("abc", 3, ""),
                    Text.Insert("", 0, "X"),
                    Text.Insert("", 0, "")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q793", () =>
            let r = try {
                    Text.Insert("abc", 4, "X")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q794", () =>
            let r = try {
                    Text.Insert("abc", -1, "X")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q795", () =>
            let r = try {
                    Text.Insert("abc", 1.5, "X")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q796", () =>
            let r = try {
                    Text.Insert("café", 3, "X"),
                    Text.Insert("café", 4, "X"),
                    Text.Insert("→→", 1, "←"),
                    Text.Insert("naïve", 2, "→")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q797", () =>
            let r = try {
                    Text.Insert(null, 0, "X"),
                    Text.Insert("abc", null, "X"),
                    Text.Insert("abc", 0, null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q798", () =>
            let r = try {
                    Text.Trim("  abc  "),
                    Text.Trim("#(tab)abc#(tab)"),
                    Text.Trim("#(cr,lf)abc#(cr,lf)"),
                    Text.Trim("#(00A0)abc#(00A0)"),
                    Text.Trim("#(2028)abc#(2028)"),
                    Text.Trim("abc")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q799", () =>
            let r = try {
                    Text.Trim("XYabcYX", {"X", "Y"}),
                    Text.Trim("XXabcXX", {"X"}),
                    Text.Trim("abc", {"X"}),
                    Text.Trim("", {"X"}),
                    Text.Trim("XYXY", {"X", "Y"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q800", () =>
            let r = try {
                    Text.Trim("→abc←", {"→", "←"}),
                    Text.Trim("→→abc←←", {"→", "←"}),
                    Text.Trim("éabcé", {"é"}),
                    Text.Trim("ßabcß", {"ß"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q801", () =>
            let r = try {
                    Text.TrimStart("XXabcXX", "X"),
                    Text.TrimEnd("XXabcXX", "X"),
                    Text.Trim("XXabcXX", "X"),
                    Text.TrimStart("  abc  "),
                    Text.TrimEnd("  abc  ")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q802", () =>
            let r = try {
                    Text.Trim("ABabcBA", {"AB"}),
                    Text.Trim("abc", {"abc"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q803", () =>
            let r = try {
                    Text.Trim(null),
                    Text.Trim("abc", null),
                    Text.Trim("XXXX", "X"),
                    Text.Trim("XXXX", {})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q804", () =>
            let r = try {
                    Text.Trim("XaXbX", "X"),
                    Text.Trim("aXbXc", "X"),
                    Text.TrimStart("aXbX", "X"),
                    Text.TrimEnd("XaXb", "X")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q805", () =>
            let r = try {
                    Text.Upper("hello"),
                    Text.Upper("café"),
                    Text.Upper("straße"),
                    Text.Upper("ß"),
                    Text.Lower("HELLO"),
                    Text.Lower("CAFÉ"),
                    Text.Lower("STRASSE")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q806", () =>
            let r = try {
                    Text.Upper("istanbul", "tr-TR"),
                    Text.Upper("ı", "tr-TR"),
                    Text.Upper("i", "tr-TR"),
                    Text.Lower("İSTANBUL", "tr-TR"),
                    Text.Lower("İ", "tr-TR"),
                    Text.Lower("I", "tr-TR")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q807", () =>
            let r = try {
                    Text.Upper("istanbul", "az-AZ"),
                    Text.Upper("ı", "az-AZ"),
                    Text.Lower("İSTANBUL", "az-AZ"),
                    Text.Lower("I", "az-AZ")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q808", () =>
            let r = try {
                    Text.Upper("istanbul", "lt-LT"),
                    Text.Lower("ISTANBUL", "lt-LT"),
                    Text.Upper("ąčęėįšų", "lt-LT")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q809", () =>
            let r = try {
                    Text.Upper(""),
                    Text.Lower(""),
                    Text.Upper("abc123!@#"),
                    Text.Lower("ABC123!@#"),
                    Text.Upper("aBcDe"),
                    Text.Lower("aBcDe")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q810", () =>
            let r = try {
                    Text.Upper(null),
                    Text.Lower(null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q811", () =>
            let r = try {
                    Text.Upper(Text.Upper("hello")) = Text.Upper("hello"),
                    Text.Lower(Text.Lower("HELLO")) = Text.Lower("HELLO"),
                    Text.Lower(Text.Upper("hello")) = "hello",
                    Text.Upper("istanbul", "tr-TR") <> Text.Upper("istanbul", "en-US"),
                    Text.Lower("İ", "tr-TR") <> Text.Lower("İ", "en-US")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q812", () =>
            let r = try {
                    Text.Lower("İ", "en-US"),
                    Text.Lower("İ", "tr-TR"),
                    Text.Lower("İ"),
                    Text.Lower("İ", "en-US") = "i",
                    Text.Lower("İ", "tr-TR") = "i"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q813", () =>
            let g = Text.NewGuid() in
            let r = try {
                    Text.Length(g) = 36,
                    Text.PositionOf(g, "-") = 8,
                    Text.PositionOf(g, "-", Occurrence.All) = {8, 13, 18, 23},
                    Text.Length(Text.Replace(g, "-", "")) = 32,
                    Text.Range(g, 14, 1) = "4"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q814", () =>
            let a = Text.NewGuid(), b = Text.NewGuid() in
            let r = try {
                    a <> b,
                    Text.Length(a) = Text.Length(b)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q815", () =>
            let g = Text.NewGuid() in
            let variantChar = Text.Range(g, 19, 1) in
            let r = try {
                    List.Contains({"8", "9", "a", "b"}, variantChar),
                    Text.Upper(g) <> g,
                    Text.Lower(g) = g
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q816", () =>
            let g = Text.NewGuid() in
            let allValidChars =
                List.AllTrue(
                    List.Transform(
                        Text.ToList(g),
                        (c) => List.Contains({"0","1","2","3","4","5","6","7","8","9","a","b","c","d","e","f","-"}, c)
                    )
                ) in
            let r = try {
                    allValidChars
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q817", () =>
            let guids = List.Generate(() => 0, (i) => i < 10, (i) => i + 1, (i) => Text.NewGuid()) in
            let r = try {
                    List.Count(List.Distinct(guids)) = 10
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q818", () =>
            let r = try {
                    Text.NewGuid("D")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q819", () =>
            let g = Text.NewGuid() in
            let parts = Text.Split(g, "-") in
            let r = try {
                    List.Count(parts) = 5,
                    Text.Length(parts{0}) = 8,
                    Text.Length(parts{1}) = 4,
                    Text.Length(parts{2}) = 4,
                    Text.Length(parts{3}) = 4,
                    Text.Length(parts{4}) = 12
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q820", () =>
            let r = try {
                    Text.PositionOf("abc", ""),
                    Text.PositionOf("", ""),
                    Text.PositionOf("", "a"),
                    Text.PositionOf("abc", "", Occurrence.All),
                    Text.PositionOf("abc", "", Occurrence.Last)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q821", () =>
            let r = try {
                    Text.PositionOf("abc", "abc"),
                    Text.PositionOf("abc", "abcd"),
                    Text.PositionOf("a", "ab"),
                    Text.PositionOf("abc", "abc", Occurrence.All)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q822", () =>
            let r = try {
                    Text.PositionOf("aaaa", "aa", Occurrence.All),
                    Text.PositionOf("aaaaa", "aa", Occurrence.All),
                    Text.PositionOf("aaa", "aa", Occurrence.All),
                    Text.PositionOf("ababab", "aba", Occurrence.All)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q823", () =>
            let r = try {
                    Text.PositionOf("Hello hello HELLO", "hello"),
                    Text.PositionOf("Hello hello HELLO", "hello", Occurrence.First, Comparer.OrdinalIgnoreCase),
                    Text.PositionOf("Hello hello HELLO", "hello", Occurrence.All, Comparer.OrdinalIgnoreCase),
                    Text.PositionOf("Hello hello HELLO", "HELLO", Occurrence.All)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q824", () =>
            let r = try {
                    Text.PositionOfAny("hello world", {"l", "o"}),
                    Text.PositionOfAny("hello world", {"l", "o"}, Occurrence.All),
                    Text.PositionOfAny("hello world", {"l", "o"}, Occurrence.Last),
                    Text.PositionOfAny("abc", {"x"}),
                    Text.PositionOfAny("abc", {"x"}, Occurrence.All)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q825", () =>
            let r = try {
                    Text.PositionOfAny("abc", {"ab"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q826", () =>
            let r = try {
                    Text.PositionOf(null, "a"),
                    Text.PositionOf("abc", null),
                    Text.PositionOfAny(null, {"a"}),
                    Text.PositionOfAny("abc", null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q827", () =>
            let r = try {
                    Text.PositionOf("abc", "z"),
                    Text.PositionOf("abc", "z", Occurrence.All),
                    Text.PositionOf("abc", "z", Occurrence.Last),
                    Text.PositionOfAny("abc", {"x", "y", "z"}),
                    Text.PositionOfAny("abc", {"x"}, Occurrence.All)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q828", () =>
            let r = try {
                    Text.Format("hello #{0}", {"world"}),
                    Text.Format("#{0} #{1}", {"hello", "world"}),
                    Text.Format("#{1} #{0}", {"hello", "world"}),
                    Text.Format("#{0}#{0}", {"X"}),
                    Text.Format("no placeholders", {"X"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q829", () =>
            let r = try {
                    Text.Format("price: #25", {"X"}),
                    Text.Format("a#b", {"X"}),
                    Text.Format("#", {"X"}),
                    Text.Format("# is literal", {"X"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q830", () =>
            let r = try {
                    Text.Format("##", {"X"}),
                    Text.Format("##{0}", {"X"}),
                    Text.Format("###{0}", {"X"}),
                    Text.Format("####", {"X"}),
                    Text.Format("a##b", {"X"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q831", () =>
            let r = try {
                    Text.Format("hello #{name}", [name="world"]),
                    Text.Format("#{a} + #{b}", [a=1, b=2]),
                    Text.Format("#{0}", [a=1])
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q832", () =>
            let r = try {
                    Text.Format("n=#{0}", {42}),
                    Text.Format("n=#{0}", {3.14}),
                    Text.Format("n=#{0}", {null}),
                    Text.Format("n=#{0}", {true}),
                    Text.Format("n=#{0}", {"text"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q833", () =>
            let r = try {
                    Text.Format("#{0}", {})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q834", () =>
            let r = try {
                    Text.Format("#{0", {"X"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q835", () =>
            let r = try {
                    Text.Combine({}),
                    Text.Combine({"a"}),
                    Text.Combine({"a", "b"}),
                    Text.Combine({"a", "b", "c"}),
                    Text.Combine({"a", "b"}, ", "),
                    Text.Combine({"a"}, ", "),
                    Text.Combine({}, ", ")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q836", () =>
            let r = try {
                    Text.Combine({"a", null, "b"}),
                    Text.Combine({null, null}),
                    Text.Combine({null}),
                    Text.Combine({"a", null, "b"}, ","),
                    Text.Combine({null, "a", null}, "-")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q837", () =>
            let r = try {
                    Text.Combine({"", ""}, ","),
                    Text.Combine({"a", ""}, ","),
                    Text.Combine({"", "a"}, ","),
                    Text.Combine({"", "", ""}, "-"),
                    Text.Combine({""}, ",")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q838", () =>
            let r = try {
                    Text.Combine({"a", "b"}, ""),
                    Text.Combine({"a", "b"}, " - "),
                    Text.Combine({"a", "b"}, "→"),
                    Text.Combine({"a", "b"}, null),
                    Text.Combine({"a", "b"}, "#(cr,lf)")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q839", () =>
            let r = try {
                    Text.Combine({"a", 42, "b"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q840", () =>
            let r = try {
                    Text.Combine(null),
                    Text.Combine("abc")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q841", () =>
            let r = try {
                    Text.Combine(Text.Split("a,b,c", ","), ",") = "a,b,c",
                    Text.Combine(Text.Split("a-b-c", "-"), "-") = "a-b-c",
                    Text.Combine({"a"}, "anything") = "a"
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q842", () =>
            let r = try {
                    Text.Reverse(""),
                    Text.Reverse("a"),
                    Text.Reverse("ab"),
                    Text.Reverse("abc"),
                    Text.Reverse("hello"),
                    Text.Reverse("aba"),
                    Text.Reverse("racecar")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q843", () =>
            let r = try {
                    Text.Reverse("café"),
                    Text.Reverse("→←"),
                    Text.Reverse("hello world"),
                    Text.Reverse("naïve"),
                    Text.Reverse("ß")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q844", () =>
            let r = try {
                    Text.Reverse("cafe#(0301)"),
                    Text.Length("cafe#(0301)"),
                    Text.Reverse("a#(0301)b#(0301)"),
                    Text.Reverse("a#(0301)")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q845", () =>
            let r = try {
                    Text.Reverse("a#(0001F600)b"),
                    Text.Length("a#(0001F600)b"),
                    Text.Reverse("#(0001F600)#(0001F601)")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q846", () =>
            let r = try {
                    Text.Reverse(Text.Reverse("hello")) = "hello",
                    Text.Reverse(Text.Reverse("café")) = "café",
                    Text.Reverse(Text.Reverse("")) = "",
                    Text.Length(Text.Reverse("hello world")) = Text.Length("hello world")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q847", () =>
            let r = try {
                    Text.Reverse(null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q848", () =>
            let r = try {
                    Text.Length(""),
                    Text.Length("a"),
                    Text.Length("café"),
                    Text.Length("cafe#(0301)"),
                    Text.Length("#(0001F600)"),
                    Text.Length("a#(0001F600)b"),
                    Text.Length("#(0001F600)#(0001F601)"),
                    Text.Length("a#(0301)")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q849", () =>
            let r = try {
                    List.Sort({3, 1, 2}),
                    List.Sort({"banana", "apple", "cherry"}),
                    List.Sort({}),
                    List.Sort({1}),
                    List.Sort({3, 3, 1, 2, 1})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q850", () =>
            let xs = {
                    [k=1, tag="A"],
                    [k=2, tag="B"],
                    [k=1, tag="C"],
                    [k=2, tag="D"],
                    [k=1, tag="E"]
                } in
            let r = try {
                    List.Sort(xs, (a, b) => Value.Compare(a[k], b[k])),
                    List.Sort(xs, each _[k])
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q851", () =>
            let r = try {
                    List.Sort({3, 1, 2}, Order.Descending),
                    List.Sort({"banana", "apple", "cherry"}, Order.Descending),
                    List.Sort({1, 2, 3}, Order.Ascending),
                    List.Sort({3, 2, 1}, Order.Ascending),
                    List.Sort({1}, Order.Descending),
                    List.Sort({}, Order.Descending)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q852", () =>
            let r = try {
                    List.Sort({"banana", "apple", "cherry", "date"}, each Text.Length(_)),
                    List.Sort({"apple", "banana", "cherry"}, each Text.Length(_)),
                    List.Sort({3.5, 1.2, 2.8, 4.0}, each Number.Round(_)),
                    List.Sort({-3, -1, 2, -5, 4}, each Number.Abs(_))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q853", () =>
            let xs = {
                    [name="A", grade=2],
                    [name="B", grade=1],
                    [name="C", grade=2],
                    [name="D", grade=1]
                } in
            let r = try {
                    List.Sort(xs, each {_[grade], _[name]})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q854", () =>
            let r = try {
                    List.Sort({3, 1, 2}, (a, b) => Value.Compare(a, b)),
                    List.Sort({3, 1, 2}, (a, b) => -Value.Compare(a, b)),
                    List.Sort({"banana", "apple"}, (a, b) => Value.Compare(Text.Lower(a), Text.Lower(b)))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q855", () =>
            let r = try {
                    List.Sort({3, null, 1, null, 2}),
                    List.Sort({"b", null, "a"}),
                    List.Sort({null, null}),
                    List.Sort({null})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q856", () =>
            let r = try {
                    List.Distinct({1, 2, 1, 3, 2}),
                    List.Distinct({"a", "b", "a", "c", "b"}),
                    List.Distinct({}),
                    List.Distinct({1}),
                    List.Distinct({null, 1, null, 2})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q857", () =>
            let r = try {
                    List.Distinct({"Apple", "apple", "APPLE", "banana"}, Comparer.OrdinalIgnoreCase),
                    List.Distinct({"a", "A"}, Comparer.OrdinalIgnoreCase),
                    List.Distinct({"a", "A"}, Comparer.Ordinal),
                    List.Distinct({"a", "A"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q858", () =>
            let r = try {
                    List.Distinct({"a", "A"}, (x, y) => Text.Lower(x) = Text.Lower(y))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q859", () =>
            let r = try {
                    List.Distinct({1, 1.0, 2, 2.0}),
                    List.Distinct({"1", 1}),
                    List.Distinct({true, false, true, true}),
                    List.Distinct({null, null, null})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q860", () =>
            let r = try {
                    List.Distinct({3, 1, 2, 1, 3, 2}),
                    List.Distinct({"c", "a", "b", "a", "c"}),
                    List.Distinct({2, 1, 2, 3, 1})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q861", () =>
            let r = try {
                    List.Distinct({[a=1], [a=2], [a=1]}),
                    List.Distinct({{1, 2}, {1, 2}, {1, 3}}),
                    List.Distinct({[a=1, b=2], [b=2, a=1]})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q862", () =>
            let r = try {
                    List.Distinct(null),
                    List.Distinct("abc")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q863", () =>
            let r = try {
                    List.Accumulate({1, 2, 3, 4}, 0, (acc, x) => acc + x),
                    List.Accumulate({1, 2, 3}, 1, (acc, x) => acc * x),
                    List.Accumulate({"a", "b", "c"}, "", (acc, x) => acc & x),
                    List.Accumulate({}, 100, (acc, x) => acc + x)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q864", () =>
            let r = try {
                    List.Accumulate({"a", "b", "c"}, [], (acc, x) => Record.AddField(acc, x, Text.Upper(x))),
                    List.Accumulate({1, 2, 3}, [count=0, sum=0], (acc, x) => [count=acc[count]+1, sum=acc[sum]+x])
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q865", () =>
            let r = try {
                    List.Accumulate({1, 2, 3}, {}, (acc, x) => acc & {x * 10}),
                    List.Accumulate({"a", "b"}, {"start"}, (acc, x) => acc & {x})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q866", () =>
            let r = try {
                    List.Accumulate({1, 2, 3}, null, (acc, x) => if acc = null then x else acc + x),
                    List.Accumulate({}, null, (acc, x) => x)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q867", () =>
            let r = try {
                    List.Accumulate({1, 2}, [nested=[count=0, items={}]],
                        (acc, x) => [nested=[count=acc[nested][count]+1, items=acc[nested][items]&{x}]])
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q868", () =>
            let r = try {
                    List.Accumulate({1, 2, 3}, "start", (acc, x) =>
                        if acc = "start" then 0 else acc + x),
                    List.Accumulate({"a", "b"}, 0, (acc, x) =>
                        if acc = 0 then x else Text.From(acc) & x)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q869", () =>
            let r = try {
                    List.Accumulate(null, 0, (acc, x) => acc + x),
                    List.Accumulate({1, 2}, 0, null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q870", () =>
            let r = try {
                    List.Generate(() => 0, (s) => s < 5, (s) => s + 1),
                    List.Generate(() => 1, (s) => s < 100, (s) => s * 2),
                    List.Generate(() => 0, (s) => s < 0, (s) => s + 1),
                    List.Generate(() => "", (s) => Text.Length(s) < 3, (s) => s & "a")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q871", () =>
            let r = try {
                    List.Generate(() => 0, (s) => s < 5, (s) => s + 1, (s) => s * s),
                    List.Generate(() => 1, (s) => s <= 10, (s) => s + 1, (s) => Text.From(s)),
                    List.Generate(() => 0, (s) => s < 0, (s) => s + 1, (s) => s * 100)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q872", () =>
            let r = try {
                    List.Generate(
                        () => [i=0, sum=0],
                        (s) => s[i] < 5,
                        (s) => [i=s[i]+1, sum=s[sum]+s[i]],
                        (s) => s[sum]
                    ),
                    List.Generate(
                        () => [a=1, b=1],
                        (s) => s[a] < 100,
                        (s) => [a=s[b], b=s[a]+s[b]],
                        (s) => s[a]
                    )
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q873", () =>
            let r = try {
                    List.Generate(() => 100, (s) => s < 5, (s) => s + 1),
                    List.Generate(() => null, (s) => s <> null, (s) => s)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q874", () =>
            let r = try {
                    List.Generate(
                        () => [n=0, found=false],
                        (s) => s[n] < 100 and not s[found],
                        (s) => [n=s[n]+1, found=Number.Mod(s[n]+1, 7) = 0],
                        (s) => s[n]
                    )
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q875", () =>
            let r = try {
                    List.Count(List.Generate(() => 0, (s) => s < 10, (s) => s + 1)) = 10,
                    List.Sum(List.Generate(() => 1, (s) => s <= 5, (s) => s + 1)) = 15
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q876", () =>
            let r = try {
                    List.Generate(() => 0, (s) => s < 3, (s) => s + 1, (s) => [n=s, doubled=s*2]),
                    List.Generate(() => 0, (s) => s < 3, (s) => s + 1, (s) => {s, s+1}),
                    List.Generate(() => 0, (s) => s < 3, (s) => s + 1, (s) => null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q877", () =>
            let xs = {10, 20, 30, 40, 50} in
            let r = try {
                    List.Range(xs, 0),
                    List.Range(xs, 2),
                    List.Range(xs, 5),
                    List.Range(xs, 0, 2),
                    List.Range(xs, 1, 3),
                    List.Range(xs, 3, 10),
                    List.Range(xs, 0, 0)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q878", () =>
            let xs = {10, 20, 30} in
            let r = try {
                    List.Range(xs, 10)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q879", () =>
            let xs = {10, 20, 30} in
            let r = try {
                    List.Range(xs, -1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q880", () =>
            let xs = {10, 20, 30, 40, 50} in
            let r = try {
                    List.Skip(xs, 0),
                    List.Skip(xs, 2),
                    List.Skip(xs, 5),
                    List.Skip(xs, 10),
                    List.Skip({}, 3),
                    List.Skip(xs)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q881", () =>
            let r = try {
                    List.Skip({10, 20, 30}, -1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q882", () =>
            let xs = {10, 20, 30, 40, 50} in
            let r = try {
                    List.FirstN(xs, 0),
                    List.FirstN(xs, 2),
                    List.FirstN(xs, 5),
                    List.FirstN(xs, 10),
                    List.LastN(xs, 0),
                    List.LastN(xs, 2),
                    List.LastN(xs, 5),
                    List.LastN(xs, 10)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q883", () =>
            let r = try {
                    List.FirstN({}, 3),
                    List.LastN({}, 3),
                    List.FirstN({1, 2, 3}, -1),
                    List.LastN({1, 2, 3}, -1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q884", () =>
            let xs = {1, 3, 5, 2, 4, 6} in
            let r = try {
                    List.FirstN(xs, each _ < 5),
                    List.FirstN(xs, each _ > 0),
                    List.FirstN(xs, each _ > 100),
                    List.LastN(xs, each _ > 3),
                    List.LastN(xs, each _ > 100)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q885", () =>
            let r = try {
                    List.Zip({{1, 2, 3}, {"a", "b", "c"}}),
                    List.Zip({{1, 2}, {"a", "b"}, {true, false}}),
                    List.Zip({{}}),
                    List.Zip({})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q886", () =>
            let r = try {
                    List.Zip({{1, 2, 3}, {"a", "b"}}),
                    List.Zip({{1}, {"a", "b", "c"}}),
                    List.Zip({{1, 2}, {}}),
                    List.Zip({{}, {1, 2}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q887", () =>
            let r = try {
                    List.Zip({{1, 2, 3}, {"a", "b", "c"}, {true, false, true}, {10, 20, 30}}),
                    List.Zip({{1}}),
                    List.Zip({{1, 2}, {"a", "b"}, {}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q888", () =>
            let r = try {
                    List.Zip(null),
                    List.Zip({null}),
                    List.Zip({{1, 2}, null}),
                    List.Zip({"not-a-list"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q889", () =>
            let pairs = List.Zip({{1, 2, 3}, {"a", "b", "c"}}) in
            let r = try {
                    List.Count(pairs) = 3,
                    List.Count(pairs{0}) = 2,
                    List.Zip(List.Zip({{1, 2}, {"a", "b"}})) = {{1, 2}, {"a", "b"}}
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q890", () =>
            let r = try {
                    List.Combine({{1, 2}, {3, 4}}),
                    List.Combine({}),
                    List.Combine({{1, 2}}),
                    List.Combine({{}, {}, {}}),
                    List.Combine({{1, 2}, "abc"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q891", () =>
            let r = try {
                    List.Combine({{{1, 2}}, {{3, 4}}}),
                    List.Combine({{null, 1}, {2, null}}),
                    List.Combine({{1, 2}, null})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q892", () =>
            let r = try {
                    List.Sum({}),
                    List.Sum({null, null, null}),
                    List.Sum({1}),
                    List.Sum({1, null, 2}),
                    List.Average({}),
                    List.Average({null, null}),
                    List.Average({1}),
                    List.Average({1, null, 3})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q893", () =>
            let r = try {
                    List.Median({}),
                    List.Median({null, null}),
                    List.Median({1}),
                    List.Median({1, 2, 3}),
                    List.Median({1, 2, 3, 4}),
                    List.Median({null, 1, 2, null, 3})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q894", () =>
            let r = try {
                    List.Mode({}),
                    List.Mode({null}),
                    List.Mode({1, 2, 2, 3, 3, 3}),
                    List.Mode({1, 2, 3}),
                    List.Modes({1, 2, 2, 3, 3}),
                    List.Modes({1, 2, 3})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q895", () =>
            let r = try {
                    List.StandardDeviation({}),
                    List.StandardDeviation({1}),
                    List.StandardDeviation({1, 1, 1}),
                    List.StandardDeviation({1, 2, 3, 4, 5}),
                    List.StandardDeviation({null, 1, 2, null, 3, 4, 5})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q896", () =>
            let r = try {
                    List.Sum({1, 2, 1/0, 3}),
                    List.Sum({1, -1/0, 1/0}),
                    List.Sum({1, 0/0, 2}),
                    List.Average({1, 1/0}),
                    List.Max({1, 1/0, 3}),
                    List.Min({1, -1/0, 3})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q897", () =>
            let r = try {
                    List.Sum({1, 2, "3"}),
                    List.Sum({1, true, 2})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q898", () =>
            let r = try {
                    List.Sum({1}),
                    List.Average({1}),
                    List.Median({5}),
                    List.Mode({5}),
                    List.StandardDeviation({5}),
                    List.Sum({null}),
                    List.Average({null}),
                    List.Median({null})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q899", () =>
            let xs = {3, 1, 4, 1, 5, 9, 2, 6} in
            let r = try {
                    List.MaxN(xs, 1),
                    List.MaxN(xs, 3),
                    List.MaxN(xs, 5),
                    List.MinN(xs, 1),
                    List.MinN(xs, 3),
                    List.MinN(xs, 5)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q900", () =>
            let xs = {3, 1, 4, 1, 5} in
            let r = try {
                    List.MaxN(xs, 0),
                    List.MaxN(xs, 100),
                    List.MinN(xs, 0),
                    List.MinN(xs, 100),
                    List.MaxN({}, 3),
                    List.MinN({}, 3)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q901", () =>
            let r = try {
                    List.MaxN({1, 2, 3}, -1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q902", () =>
            let r = try {
                    List.MaxN({1, 5, 5, 3, 5}, 3),
                    List.MinN({1, 5, 5, 3, 5}, 3),
                    List.MaxN({null, 1, null, 2, 3}, 2),
                    List.MinN({null, 1, null, 2, 3}, 2),
                    List.MaxN({null, null}, 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q903", () =>
            let r = try {
                    List.Numbers(1, 5),
                    List.Numbers(0, 0),
                    List.Numbers(5, 3),
                    List.Numbers(1, 5, 2),
                    List.Numbers(10, 4, -1),
                    List.Numbers(0, 3, 0.5),
                    List.Numbers(1, 5, -2)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q904", () =>
            let r = try {
                    List.Numbers(0, -1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q905", () =>
            let r = try {
                    List.Numbers(0, 5) = {0, 1, 2, 3, 4},
                    List.Sum(List.Numbers(1, 100)) = 5050,
                    List.Numbers(1.5, 3) = {1.5, 2.5, 3.5}
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q906", () =>
            let r = try {
                    List.Dates(#date(2026, 1, 1), 5, #duration(1, 0, 0, 0)),
                    List.Dates(#date(2026, 1, 1), 0, #duration(1, 0, 0, 0)),
                    List.Dates(#date(2026, 1, 1), 1, #duration(1, 0, 0, 0)),
                    List.Dates(#date(2026, 12, 30), 5, #duration(1, 0, 0, 0))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q907", () =>
            let r = try {
                    List.Dates(#date(2026, 1, 5), 5, #duration(-1, 0, 0, 0)),
                    List.Dates(#date(2026, 3, 1), 3, #duration(-30, 0, 0, 0))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q908", () =>
            let r = try {
                    List.DateTimes(#datetime(2026, 1, 1, 0, 0, 0), 4, #duration(0, 6, 0, 0)),
                    List.DateTimes(#datetime(2026, 1, 1, 0, 0, 0), 3, #duration(0, 0, 30, 0)),
                    List.DateTimes(#datetime(2026, 1, 1, 23, 30, 0), 3, #duration(0, 0, 30, 0))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q909", () =>
            let r = try {
                    List.Durations(#duration(0, 0, 0, 0), 5, #duration(0, 1, 0, 0)),
                    List.Durations(#duration(0, 0, 0, 0), 3, #duration(0, 0, 15, 0)),
                    List.Durations(#duration(1, 0, 0, 0), 4, #duration(0, 12, 0, 0))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q910", () =>
            let r = try {
                    List.Dates(#date(2026, 1, 1), 0, #duration(1, 0, 0, 0)),
                    List.Dates(#date(2026, 1, 1), -1, #duration(1, 0, 0, 0))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q911", () =>
            let r = try {
                    List.Dates(#date(2026, 1, 1), 3, #duration(0, 0, 0, 0))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q912", () =>
            let r = try {
                    List.Dates(#date(2024, 2, 28), 3, #duration(1, 0, 0, 0)),
                    List.Dates(#date(2025, 2, 28), 3, #duration(1, 0, 0, 0)),
                    List.Dates(#date(2024, 12, 30), 5, #duration(1, 0, 0, 0))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q913", () =>
            let r = try {
                    List.Buffer({1, 2, 3}),
                    List.Buffer({}),
                    List.Buffer({"a", "b"}),
                    List.Buffer({null, 1, null}),
                    List.Buffer({{1, 2}, {3, 4}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q914", () =>
            let r = try {
                    List.Buffer({1, 2, 3}) = {1, 2, 3},
                    List.Count(List.Buffer({1, 2, 3})) = 3,
                    List.Sum(List.Buffer({10, 20, 30})) = 60,
                    List.Transform(List.Buffer({1, 2, 3}), each _ * 2) = {2, 4, 6}
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q915", () =>
            let r = try {
                    List.Buffer(List.Generate(() => 0, (s) => s < 5, (s) => s + 1)),
                    List.Buffer(List.Numbers(0, 5)),
                    List.Sum(List.Buffer(List.Numbers(1, 10))) = 55
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q916", () =>
            let r = try {
                    List.Buffer(null),
                    List.Buffer("abc"),
                    List.Buffer(42)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q917", () =>
            let xs = List.Buffer({5, 2, 8, 1, 9}) in
            let r = try {
                    List.Reverse(xs),
                    List.FirstN(xs, 3),
                    List.LastN(xs, 2),
                    List.Sort(xs)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q918", () =>
            let r = try {
                    List.Buffer(List.Buffer({1, 2, 3})),
                    List.Buffer(List.Buffer({})) = {}
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q919", () =>
            let r = try {
                    List.Reverse(List.Buffer({1, 2, 3, 4, 5})),
                    List.Reverse(List.Buffer({})),
                    List.Reverse(List.Buffer({"a"})),
                    List.Reverse(List.Buffer({null, 1, null}))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q920", () =>
            let a = Number.Random(), b = Number.Random(), c = Number.Random() in
            let r = try {
                    a >= 0 and a < 1,
                    b >= 0 and b < 1,
                    c >= 0 and c < 1,
                    not (a = b and b = c)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q921", () =>
            let r = try {
                    Number.RandomBetween(0, 1) >= 0,
                    Number.RandomBetween(0, 1) <= 1,
                    Number.RandomBetween(-10, 10) >= -10,
                    Number.RandomBetween(-10, 10) <= 10,
                    Number.RandomBetween(100, 100) = 100
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q922", () =>
            let r = try {
                    List.Count(List.Random(10)) = 10,
                    List.Count(List.Random(0)) = 0,
                    List.Random(5, 42) = List.Random(5, 42),
                    List.Random(5, 42) <> List.Random(5, 99)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q923", () =>
            let xs = List.Transform(List.Numbers(1, 1000), each Number.RandomBetween(0, 100)) in
            let r = try {
                    List.Max(xs) <= 100,
                    List.Min(xs) >= 0,
                    List.Average(xs) > 30 and List.Average(xs) < 70,
                    List.Count(List.Distinct(xs)) > 500
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q924", () =>
            let r = try {
                    Number.RandomBetween(10, 0),
                    Number.RandomBetween(5, 5),
                    Number.RandomBetween(-1, 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q925", () =>
            let r = try {
                    Number.RandomBetween(null, 10),
                    Number.RandomBetween(0, null),
                    List.Random(null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q926", () =>
            let r = try {
                    Number.Random() <> Number.Random(),
                    Number.Random() >= 0,
                    Number.RandomBetween(0, 1) <> Number.RandomBetween(0, 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q927", () =>
            let r = try {
                    List.Transform({1, 2, 3}, each _ * 2),
                    List.Transform({}, each _ * 2),
                    List.Transform({"a", "b"}, each Text.Upper(_))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q928", () =>
            let r = try {
                    List.Transform({"a", "b", "c"}, (item, idx) => Text.From(idx) & "=" & item),
                    List.Transform({10, 20, 30}, (v, i) => v + i),
                    List.Transform({}, (v, i) => v + i)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q929", () =>
            let r = try {
                    List.Transform({100, 200, 300}, (v, i) => [pos=i, val=v]),
                    List.Transform(List.Numbers(1, 5), (n, i) => n * 100 + i)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q930", () =>
            let r = try {
                    List.Transform({1, 2, 3}, () => 99),
                    List.Transform({1, 2, 3}, (a, b, c) => a + b + c)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q931", () =>
            let r = try {
                    List.Transform(null, each _),
                    List.Transform({1, 2}, null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q932", () =>
            let r = try {
                    List.Select({10, 20, 30, 40}, each _ > 15),
                    List.Select({}, each _ > 0),
                    List.Select({10, 20, 30, 40}, (v, i) => i > 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q933", () =>
            let r = try {
                    List.RemoveItems({"a", "B", "c", "D"}, {"b", "d"}),
                    List.RemoveItems({"a", "B", "c", "D"}, {"b", "d"}, Comparer.OrdinalIgnoreCase),
                    List.RemoveItems({1, 2, 3, 4}, {2, 4}),
                    List.RemoveItems({}, {1, 2})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q934", () =>
            let t = Table.FromRecords({
                    [k="a", v=1],
                    [k="b", v=2],
                    [k="a", v=3],
                    [k="b", v=4]
                }) in
            let r = try {
                    Table.Group(t, "k", {{"sum", each List.Sum([v]), type number}}),
                    Table.Group(t, "k", {{"count", each Table.RowCount(_), type number}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q935", () =>
            let t = Table.FromRecords({
                    [a=1, b="x", v=10],
                    [a=1, b="y", v=20],
                    [a=1, b="x", v=30],
                    [a=2, b="x", v=40]
                }) in
            let r = try {
                    Table.Group(t, {"a", "b"}, {{"sum", each List.Sum([v]), type number}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q936", () =>
            let t = Table.FromRecords({
                    [k="a", v=1],
                    [k="a", v=2],
                    [k="b", v=3],
                    [k="a", v=4]
                }) in
            let r = try {
                    Table.Group(t, "k", {{"sum", each List.Sum([v]), type number}}, GroupKind.Global),
                    Table.Group(t, "k", {{"sum", each List.Sum([v]), type number}}, GroupKind.Local)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q937", () =>
            let t = Table.FromRecords({
                    [k="a", v=10],
                    [k="b", v=20],
                    [k="a", v=30],
                    [k="b", v=40]
                }) in
            let r = try {
                    Table.Group(t, "k", {
                        {"sum",   each List.Sum([v]),     type number},
                        {"avg",   each List.Average([v]), type number},
                        {"count", each Table.RowCount(_), type number},
                        {"items", each [v],               type list}
                    })
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q938", () =>
            let empty = Table.FromRecords({}) in
            let r = try {
                    Table.Group(empty, "k", {{"sum", each 0, type number}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q939", () =>
            let t = Table.FromRecords({
                    [k=null, v=1],
                    [k="a", v=2],
                    [k=null, v=3],
                    [k="a", v=4]
                }) in
            let r = try {
                    Table.Group(t, "k", {{"sum", each List.Sum([v]), type number}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q940", () =>
            let t = Table.FromRecords({[k="a", v=1]}) in
            let r = try {
                    Table.Group(t, "missing", {{"sum", each List.Sum([v]), type number}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q941", () =>
            let t = Table.FromRecords({
                    [name="b", v=2],
                    [name="a", v=3],
                    [name="c", v=1]
                }) in
            let r = try {
                    Table.Sort(t, "name"),
                    Table.Sort(t, "v")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q942", () =>
            let t = Table.FromRecords({
                    [name="b", v=2],
                    [name="a", v=3],
                    [name="c", v=1]
                }) in
            let r = try {
                    Table.Sort(t, {"v", Order.Descending}),
                    Table.Sort(t, {"name", Order.Descending}),
                    Table.Sort(t, {"v", Order.Ascending})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q943", () =>
            let t = Table.FromRecords({
                    [g=1, v=30],
                    [g=2, v=10],
                    [g=1, v=20],
                    [g=2, v=40]
                }) in
            let r = try {
                    Table.Sort(t, {{"g", Order.Ascending}, {"v", Order.Descending}}),
                    Table.Sort(t, {{"g", Order.Descending}, {"v", Order.Ascending}}),
                    Table.Sort(t, {"g", "v"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q944", () =>
            let t = Table.FromRecords({
                    [k=1, tag="A"],
                    [k=2, tag="B"],
                    [k=1, tag="C"],
                    [k=2, tag="D"],
                    [k=1, tag="E"]
                }) in
            let r = try {
                    Table.Sort(t, "k")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q945", () =>
            let t = Table.FromRecords({
                    [v=2],
                    [v=null],
                    [v=1],
                    [v=null],
                    [v=3]
                }) in
            let r = try {
                    Table.Sort(t, "v"),
                    Table.Sort(t, {"v", Order.Descending})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q946", () =>
            let t = Table.FromRecords({
                    [name="Apple"],
                    [name="banana"],
                    [name="Cherry"]
                }) in
            let r = try {
                    Table.Sort(t, "name"),
                    Table.Sort(t, {"name", (a, b) => Value.Compare(Text.Lower(a), Text.Lower(b))})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q947", () =>
            let t = Table.FromRecords({[a=1, b=2]}) in
            let r = try {
                    Table.Sort(Table.FromRecords({}), "a"),
                    Table.Sort(t, "a"),
                    Table.Sort(t, "missing")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q948", () =>
            let t = Table.FromRecords({
                    [n=1, v=10],
                    [n=2, v=20],
                    [n=3, v=30]
                }) in
            let r = try {
                    Table.SelectRows(t, each [n] > 1),
                    Table.SelectRows(t, each [v] >= 20),
                    Table.SelectRows(t, each false),
                    Table.SelectRows(t, each true)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q949", () =>
            let t = Table.FromRecords({
                    [n=1, v=10],
                    [n=2, v=null],
                    [n=3, v=30]
                }) in
            let r = try {
                    Table.SelectRows(t, each [v] > 15)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q950", () =>
            let t = Table.FromRecords({
                    [n=1],
                    [n=2],
                    [n=3]
                }) in
            let r = try {
                    Table.SelectRows(t, each null),
                    Table.SelectRows(t, each if [n] = 2 then null else [n] > 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q951", () =>
            let t = Table.FromRecords({}) in
            let r = try {
                    Table.SelectRows(t, each true),
                    Table.SelectRows(t, each false)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q952", () =>
            let t = Table.FromRecords({[n=1], [n=2]}) in
            let r = try {
                    Table.SelectRows(t, each [n]),
                    Table.SelectRows(t, each "yes")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q953", () =>
            let t = Table.FromRecords({
                    [a=1, b=10],
                    [a=2, b=20],
                    [a=1, b=30],
                    [a=2, b=40]
                }) in
            let r = try {
                    Table.SelectRows(t, each [a] = 1 and [b] > 15),
                    Table.SelectRows(t, each [a] = 1 or [b] > 30),
                    Table.SelectRows(t, each not ([a] = 1))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q954", () =>
            let t = Table.FromRecords({[n=1]}) in
            let r = try {
                    Table.SelectRows(t, null),
                    Table.SelectRows(t, "string-not-function"),
                    Table.SelectRows(null, each true)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q955", () =>
            let t = Table.FromRecords({[a=1, b=2], [a=3, b=4]}) in
            let r = try {
                    Table.AddColumn(t, "c", each [a] + [b]),
                    Table.AddColumn(t, "c", each [a] * [b]),
                    Table.AddColumn(t, "c", each Text.From([a]))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q956", () =>
            let t = Table.FromRecords({[a=1], [a=2], [a=3]}) in
            let r = try {
                    Table.AddColumn(t, "doubled",  each [a] * 2, type number),
                    Table.AddColumn(t, "text",     each Text.From([a]), type text),
                    Table.AddColumn(t, "isOdd",    each Number.Mod([a], 2) = 1, type logical),
                    Table.AddColumn(t, "now",      each #date(2026, 1, 1), type date)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q957", () =>
            let t = Table.FromRecords({[a=1]}) in
            let r = try {
                    Table.AddColumn(t, "c", each "not a number", type number)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q958", () =>
            let t = Table.FromRecords({[a=1], [a=2]}) in
            let r = try {
                    Table.AddColumn(t, "nullable", each null, type nullable number),
                    Table.AddColumn(t, "list", each {[a], [a]*2}, type list)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q959", () =>
            let t = Table.FromRecords({[a=1, b=2]}) in
            let r = try {
                    Table.AddColumn(t, "a", each 99)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q960", () =>
            let t = Table.FromRecords({}) in
            let r = try {
                    Table.AddColumn(t, "c", each 1, type number)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q961", () =>
            let t = Table.FromRecords({[a=1], [a=0], [a=2]}) in
            let r = try {
                    Table.AddColumn(t, "div", each 1 / [a])
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q962", () =>
            let t = Table.FromRecords({[v=10], [v=20], [v=30]}) in
            let r = try {
                    Table.AddIndexColumn(t, "Index"),
                    Table.AddIndexColumn(t, "Idx", 1),
                    Table.AddIndexColumn(t, "Idx", 100, 10)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q963", () =>
            let t = Table.FromRecords({[v=10], [v=20], [v=30]}) in
            let r = try {
                    Table.AddIndexColumn(t, "Index", -1, 1),
                    Table.AddIndexColumn(t, "Index", -10, 5)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q964", () =>
            let t = Table.FromRecords({[v=10], [v=20], [v=30]}) in
            let r = try {
                    Table.AddIndexColumn(t, "Index", 0, -1),
                    Table.AddIndexColumn(t, "Index", 100, -10)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q965", () =>
            let t = Table.FromRecords({[v=10], [v=20], [v=30]}) in
            let r = try {
                    Table.AddIndexColumn(t, "Index", 0, 0.5),
                    Table.AddIndexColumn(t, "Index", 0.5, 1)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q966", () =>
            let t = Table.FromRecords({[v=10], [v=20], [v=30]}) in
            let r = try {
                    Table.AddIndexColumn(t, "Index", 5, 0)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q967", () =>
            let empty = Table.FromRecords({}) in
            let t = Table.FromRecords({[Index=99]}) in
            let r = try {
                    Table.AddIndexColumn(empty, "Index"),
                    Table.AddIndexColumn(t, "Index")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q968", () =>
            let t = Table.FromRecords({[v=1]}) in
            let r = try {
                    Table.AddIndexColumn(t, "Index", null, null),
                    Table.AddIndexColumn(t, "Index", null, 2),
                    Table.AddIndexColumn(t, "Index", 5, null)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q969", () =>
            let t = Table.FromRecords({
                    [k=1, r=[a=10, b=20]],
                    [k=2, r=[a=30, b=40]]
                }) in
            let r = try {
                    Table.ExpandRecordColumn(t, "r", {"a", "b"}),
                    Table.ExpandRecordColumn(t, "r", {"a"}),
                    Table.ExpandRecordColumn(t, "r", {"a", "b"}, {"a2", "b2"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q970", () =>
            let t = Table.FromRecords({
                    [k=1, r=[a=10, b=20]],
                    [k=2, r=null],
                    [k=3, r=[a=30]]
                }) in
            let r = try {
                    Table.ExpandRecordColumn(t, "r", {"a", "b"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q971", () =>
            let t = Table.FromRecords({
                    [k=1, lst={10, 20}],
                    [k=2, lst={30}],
                    [k=3, lst={}]
                }) in
            let r = try {
                    Table.ExpandListColumn(t, "lst")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q972", () =>
            let t = Table.FromRecords({
                    [k=1, lst={10, 20}],
                    [k=2, lst=null]
                }) in
            let r = try {
                    Table.ExpandListColumn(t, "lst")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q973", () =>
            let inner1 = Table.FromRecords({[x=1, y=2], [x=3, y=4]}) in
            let inner2 = Table.FromRecords({[x=5, y=6]}) in
            let t = Table.FromRecords({
                    [k=1, tbl=inner1],
                    [k=2, tbl=inner2]
                }) in
            let r = try {
                    Table.ExpandTableColumn(t, "tbl", {"x", "y"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q974", () =>
            let t = Table.FromRecords({[k=1, r=[a=10]]}) in
            let r = try {
                    Table.ExpandRecordColumn(t, "missing", {"a"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q975", () =>
            let t = Table.FromRecords({
                    [k=1, r=[a=[x=1, y=2]]],
                    [k=2, r=[a=[x=3, y=4]]]
                }) in
            let r = try {
                    Table.ExpandRecordColumn(t, "r", {"a"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q976", () =>
            let t = Table.FromRecords({
                    [k="A", attr="x", val=1],
                    [k="A", attr="y", val=2],
                    [k="B", attr="x", val=3],
                    [k="B", attr="y", val=4]
                }) in
            let r = try {
                    Table.Pivot(t, {"x", "y"}, "attr", "val")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q977", () =>
            let t = Table.FromRecords({
                    [k="A", attr="x", val=1],
                    [k="A", attr="x", val=10],
                    [k="B", attr="x", val=3]
                }) in
            let r = try {
                    Table.Pivot(t, {"x"}, "attr", "val"),
                    Table.Pivot(t, {"x"}, "attr", "val", List.Sum)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q978", () =>
            let t = Table.FromRecords({
                    [k="A", attr=null, val=1],
                    [k="A", attr="x", val=2],
                    [k="B", attr=null, val=3]
                }) in
            let r = try {
                    Table.Pivot(t, {"x"}, "attr", "val")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q979", () =>
            let t = Table.FromRecords({
                    [k="A", x=1, y=2],
                    [k="B", x=3, y=4]
                }) in
            let r = try {
                    Table.Unpivot(t, {"x", "y"}, "attr", "val"),
                    Table.Unpivot(t, {"x"}, "attr", "val")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q980", () =>
            let t = Table.FromRecords({
                    [k="A", x=1, y=null],
                    [k="B", x=null, y=4]
                }) in
            let r = try {
                    Table.Unpivot(t, {"x", "y"}, "attr", "val")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q981", () =>
            let t = Table.FromRecords({
                    [k="A", x=1, y=2, z=3],
                    [k="B", x=4, y=5, z=6]
                }) in
            let r = try {
                    Table.UnpivotOtherColumns(t, {"k"}, "attr", "val")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q982", () =>
            let t = Table.FromRecords({
                    [k="A", x=1, y=2],
                    [k="B", x=3, y=4]
                }) in
            let unp = Table.Unpivot(t, {"x", "y"}, "attr", "val") in
            let r = try {
                    Table.Pivot(unp, {"x", "y"}, "attr", "val")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q983", () =>
            let a = Table.FromRecords({[k=1, v="a1"], [k=2, v="a2"], [k=3, v="a3"]}) in
            let b = Table.FromRecords({[k=2, w="b2"], [k=3, w="b3"], [k=4, w="b4"]}) in
            let r = try {
                    Table.Join(a, "k", b, "k", JoinKind.Inner)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q984", () =>
            let a = Table.FromRecords({[k=1, v="a1"], [k=2, v="a2"]}) in
            let b = Table.FromRecords({[k=2, w="b2"], [k=3, w="b3"]}) in
            let r = try {
                    Table.Join(a, "k", b, "k", JoinKind.LeftOuter),
                    Table.Join(a, "k", b, "k", JoinKind.RightOuter),
                    Table.Join(a, "k", b, "k", JoinKind.FullOuter)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q985", () =>
            let a = Table.FromRecords({[k=1, v="a1"], [k=2, v="a2"], [k=3, v="a3"]}) in
            let b = Table.FromRecords({[k=2, w="b2"], [k=4, w="b4"]}) in
            let r = try {
                    Table.Join(a, "k", b, "k", JoinKind.LeftAnti),
                    Table.Join(a, "k", b, "k", JoinKind.RightAnti)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q986", () =>
            let a = Table.FromRecords({
                    [g="g1", k=1, v="a11"],
                    [g="g1", k=2, v="a12"],
                    [g="g2", k=1, v="a21"]
                }) in
            let b = Table.FromRecords({
                    [g="g1", k=1, w="b11"],
                    [g="g1", k=2, w="b12"],
                    [g="g3", k=1, w="b31"]
                }) in
            let r = try {
                    Table.Join(a, {"g", "k"}, b, {"g", "k"}, JoinKind.Inner)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q987", () =>
            let a = Table.FromRecords({[k=null, v="a-null"], [k=1, v="a1"]}) in
            let b = Table.FromRecords({[k=null, w="b-null"], [k=1, w="b1"]}) in
            let r = try {
                    Table.Join(a, "k", b, "k", JoinKind.Inner),
                    Table.Join(a, "k", b, "k", JoinKind.LeftOuter)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q988", () =>
            let nonEmpty = Table.FromRecords({[k=1, v="a"]}) in
            let empty = Table.FromRecords({}) in
            let r = try {
                    Table.Join(nonEmpty, "k", empty, "k", JoinKind.LeftOuter),
                    Table.Join(empty, "k", nonEmpty, "k", JoinKind.LeftOuter),
                    Table.Join(empty, "k", empty, "k", JoinKind.Inner)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q989", () =>
            let a = Table.FromRecords({[k=1, v="a1a"], [k=1, v="a1b"]}) in
            let b = Table.FromRecords({[k=1, w="b1a"], [k=1, w="b1b"]}) in
            let r = try {
                    Table.Join(a, "k", b, "k", JoinKind.Inner)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q990", () =>
            let a = Table.FromRecords({[k=1, v="a1"], [k=2, v="a2"], [k=3, v="a3"]}) in
            let b = Table.FromRecords({[k=2, w="b2"], [k=3, w="b3"], [k=4, w="b4"]}) in
            let r = try {
                    Table.NestedJoin(a, "k", b, "k", "nested", JoinKind.Inner)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q991", () =>
            let a = Table.FromRecords({[k=1, v="a1"], [k=2, v="a2"]}) in
            let b = Table.FromRecords({[k=2, w="b2"], [k=3, w="b3"]}) in
            let r = try {
                    Table.NestedJoin(a, "k", b, "k", "nested", JoinKind.LeftOuter)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q992", () =>
            let a = Table.FromRecords({[k=1, v="a1"], [k=2, v="a2"]}) in
            let b = Table.FromRecords({[k=2, w="b2"], [k=3, w="b3"]}) in
            let nested = Table.NestedJoin(a, "k", b, "k", "tbl", JoinKind.Inner) in
            let r = try {
                    Table.ExpandTableColumn(nested, "tbl", {"w"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q993", () =>
            let a = Table.FromRecords({
                    [g="x", k=1, v="a1"],
                    [g="x", k=2, v="a2"],
                    [g="y", k=1, v="a3"]
                }) in
            let b = Table.FromRecords({
                    [g="x", k=1, w="b1"],
                    [g="x", k=2, w="b2"]
                }) in
            let r = try {
                    Table.NestedJoin(a, {"g", "k"}, b, {"g", "k"}, "nested", JoinKind.LeftOuter)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q994", () =>
            let a = Table.FromRecords({[k=null, v="a1"], [k=1, v="a2"]}) in
            let b = Table.FromRecords({[k=null, w="b1"], [k=1, w="b2"]}) in
            let r = try {
                    Table.NestedJoin(a, "k", b, "k", "nested", JoinKind.LeftOuter)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q995", () =>
            let a = Table.FromRecords({[k=1, v="a1"], [k=2, v="a2"], [k=3, v="a3"]}) in
            let b = Table.FromRecords({[k=2, w="b2"]}) in
            let r = try {
                    Table.NestedJoin(a, "k", b, "k", "nested", JoinKind.LeftAnti),
                    Table.NestedJoin(a, "k", b, "k", "nested", JoinKind.RightAnti)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q996", () =>
            let a = Table.FromRecords({[k=1, v="a1"], [k=2, v="a2"]}) in
            let b = Table.FromRecords({[k=2, w="b2"], [k=3, w="b3"]}) in
            let r = try {
                    Table.NestedJoin(a, "k", b, "k", "nested", JoinKind.RightOuter),
                    Table.NestedJoin(a, "k", b, "k", "nested", JoinKind.FullOuter)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q997", () =>
            let t = Table.FromRecords({
                    [k=1, v="A"],
                    [k=2, v=null],
                    [k=3, v=null],
                    [k=4, v="B"],
                    [k=5, v=null]
                }) in
            let r = try {
                    Table.FillDown(t, {"v"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q998", () =>
            let t = Table.FromRecords({
                    [k=1, v=null],
                    [k=2, v=null],
                    [k=3, v="A"],
                    [k=4, v=null]
                }) in
            let r = try {
                    Table.FillDown(t, {"v"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q999", () =>
            let t = Table.FromRecords({
                    [k=1, v=null],
                    [k=2, v=null],
                    [k=3, v=null]
                }) in
            let r = try {
                    Table.FillDown(t, {"v"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1000", () =>
            let t = Table.FromRecords({
                    [k=1, v=null],
                    [k=2, v="A"],
                    [k=3, v=null],
                    [k=4, v="B"],
                    [k=5, v=null]
                }) in
            let r = try {
                    Table.FillUp(t, {"v"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1001", () =>
            let t = Table.FromRecords({
                    [k=1, a="A1", b="B1"],
                    [k=2, a=null, b=null],
                    [k=3, a=null, b="B3"],
                    [k=4, a="A4", b=null]
                }) in
            let r = try {
                    Table.FillDown(t, {"a", "b"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1002", () =>
            let empty = Table.FromRecords({}) in
            let t = Table.FromRecords({[a=1]}) in
            let r = try {
                    Table.FillDown(empty, {"a"}),
                    Table.FillDown(t, {"missing"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1003", () =>
            let t = Table.FromRecords({
                    [v="A"],
                    [v="B"],
                    [v="C"]
                }) in
            let r = try {
                    Table.FillDown(t, {"v"}) = t,
                    Table.FillUp(t, {"v"}) = t
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1004", () =>
            let t = Table.FromRecords({
                    [k=1, v="A"],
                    [k=2, v="B"],
                    [k=3, v="A"]
                }) in
            let r = try {
                    Table.ReplaceValue(t, "A", "X", Replacer.ReplaceValue, {"v"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1005", () =>
            let t = Table.FromRecords({
                    [a="A", b="A"],
                    [a="B", b="A"],
                    [a="A", b="C"]
                }) in
            let r = try {
                    Table.ReplaceValue(t, "A", "X", Replacer.ReplaceValue, {"a", "b"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1006", () =>
            let t = Table.FromRecords({
                    [v="hello world"],
                    [v="say hello"],
                    [v="no match"]
                }) in
            let r = try {
                    Table.ReplaceValue(t, "hello", "HI", Replacer.ReplaceText, {"v"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1007", () =>
            let t = Table.FromRecords({
                    [v="A"],
                    [v=null],
                    [v="B"],
                    [v=null]
                }) in
            let r = try {
                    Table.ReplaceValue(t, null, "X", Replacer.ReplaceValue, {"v"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1008", () =>
            let t = Table.FromRecords({[a=1], [a=0], [a=2]}) in
            let withErrs = Table.AddColumn(t, "div", each Number.IntegerDivide(1, [a])) in
            let r = try {
                    Table.ReplaceErrorValues(withErrs, {{"div", -1}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1009", () =>
            let t = Table.FromRecords({[a=0], [a=0], [a=0]}) in
            let withErrs = Table.AddColumn(t, "div", each Number.IntegerDivide(1, [a])) in
            let r = try {
                    Table.ReplaceErrorValues(withErrs, {{"div", "ZERO"}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1010", () =>
            let t = Table.FromRecords({[v="A"], [v="B"]}) in
            let r = try {
                    Table.ReplaceValue(t, "Z", "X", Replacer.ReplaceValue, {"v"}) = t,
                    Table.ReplaceValue(t, "A", "A", Replacer.ReplaceValue, {"v"}) = t
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1011", () =>
            let t = Table.FromRecords({[v="1"], [v="2.5"], [v="-3"]}) in
            let r = try {
                    Table.TransformColumnTypes(t, {{"v", type number}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1012", () =>
            let t = Table.FromRecords({[v="1,5"], [v="2,75"], [v="-3,14"]}) in
            let r = try {
                    Table.TransformColumnTypes(t, {{"v", type number}}, "de-DE"),
                    Table.TransformColumnTypes(t, {{"v", type number}}, "en-US")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1013", () =>
            let t = Table.FromRecords({[v="1234,5"], [v="1234,56"]}) in
            let r = try {
                    Table.TransformColumnTypes(t, {{"v", type number}}, "fr-FR")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1014", () =>
            let t = Table.FromRecords({[d="15.06.2026"], [d="01.01.2026"]}) in
            let r = try {
                    Table.TransformColumnTypes(t, {{"d", type date}}, "de-DE")
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1015", () =>
            let t = Table.FromRecords({
                    [a="1", b="2.5"],
                    [a="3", b="4.0"]
                }) in
            let r = try {
                    Table.TransformColumnTypes(t, {{"a", type number}, {"b", type number}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1016", () =>
            let t = Table.FromRecords({
                    [v="1"],
                    [v="not-a-number"],
                    [v="3"]
                }) in
            let r = try {
                    Table.TransformColumnTypes(t, {{"v", type number}})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1017", () =>
            let t = Table.FromRecords({[v=1.5], [v=2.0], [v=-3.14]}) in
            let asText = Table.TransformColumnTypes(t, {{"v", type text}}) in
            let r = try {
                    Table.TransformColumnTypes(asText, {{"v", type number}}) = t
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1018", () =>
            let r = try {
                    Table.FromRecords({[a=1, b=2], [a=3, b=4]}),
                    Table.FromRecords({}),
                    Table.FromRecords({[a=1]}),
                    Table.FromRecords({[a=1, b=2], [a=3, c=4]})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1019", () =>
            let r = try {
                    Table.FromRows({{1, "a"}, {2, "b"}}, {"k", "v"}),
                    Table.FromRows({}, {"k", "v"}),
                    Table.FromRows({{1, "a"}}, {"k", "v"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1020", () =>
            let r = try {
                    Table.FromList({1, 2, 3}, null, {"v"}),
                    Table.FromList({"a", "b"}, null, {"v"}),
                    Table.FromList({}, null, {"v"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1021", () =>
            let r = try {
                    Table.FromColumns({{1, 2, 3}, {"a", "b", "c"}}, {"k", "v"}),
                    Table.FromColumns({{1, 2}, {"a", "b"}, {true, false}}, {"k", "v", "f"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1022", () =>
            let r = try {
                    Table.FromColumns({{1, 2, 3}, {"a", "b"}}, {"k", "v"}),
                    Table.FromColumns({{1, 2}, {"a", "b", "c"}}, {"k", "v"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1023", () =>
            let r = try {
                    Table.FromRows({{1, 2}, {3}}, {"a", "b"}),
                    Table.FromRows({{1, 2, 3}}, {"a", "b"})
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]),
        SafeSerialize("q1024", () =>
            let original = Table.FromRecords({[a=1, b="x"], [a=2, b="y"]}) in
            let rows = Table.ToRows(original) in
            let r = try {
                    Table.FromRows(rows, {"a", "b"}) = original
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]])
    },

    Catalog = Table.FromRecords(cases)
in
    Catalog
