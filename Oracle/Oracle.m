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
        else if v is number then Number.ToText(v, "G", "en-US")
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
                #table({"A"}, {{2}})}))
    },

    Catalog = Table.FromRecords(cases)
in
    Catalog
