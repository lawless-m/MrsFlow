// Oracle.m — catalog of test M expressions to evaluate against real
// Power Query (Excel host) and against mrsflow, then compare.
//
// This file is NOT a single M expression — each labelled section is a
// separate query body. Workflow:
//
//   1. In Oracle.xlsx, create one Power Query per `// q<N>:` section
//      below. Name each query exactly `q1`, `q2`, …
//   2. For each query, paste the expression body (the lines between
//      `// q<N>: …` and the next `//` header) into the Advanced Editor.
//   3. "Load To" each query — Excel writes it to a Table object on a
//      sheet. The Table's name becomes a workbook Name, which
//      `QueryOracle.ps1` enumerates and dumps.
//   4. Compare against the `mrsflow:` line under each case below.
//      For CLI-side regeneration: split each section into `cases/q<N>.m`,
//      then `mrsflow q1.m q2.m … --out q1 --out q2 …`.

// q1: cycle detection — record's field references itself by bare name.
//     mrsflow: EVAL ERROR (cyclic reference)
[X = X][X]

// q2: cycle detection — mutual reference between sibling fields.
//     mrsflow: EVAL ERROR (cyclic reference)
[a = b, b = a][a]

// q3: Date.ToText dd-MMM-yy.
//     mrsflow: "15-Jun-26"
Date.ToText(#date(2026, 6, 15), "dd-MMM-yy")

// q4: Date.ToText long English form — locale-dependent in PQ?
//     mrsflow: "Monday, January 5, 2026" (always English; chrono-default)
Date.ToText(#date(2026, 1, 5), "dddd, MMMM d, yyyy")

// q5: Date.ToText 2-digit year and zero-padded MM/dd.
//     mrsflow: "26.06.05"
Date.ToText(#date(2026, 6, 5), "yy.MM.dd")

// q6: Date.ToText unpadded M/d.
//     mrsflow: "6/5"
Date.ToText(#date(2026, 6, 5), "M/d")

// q7: Table.PromoteHeaders with PromoteAllScalars on heterogeneous
//     scalar header row.
//     mrsflow: columns "1.5" / "true" (lower-case logical, Rust f64 repr)
Table.PromoteHeaders(
    #table({"A","B"}, {{1.5, true}, {"x", "y"}}),
    [PromoteAllScalars=true])

// q8: Text.ToBinary with the BinaryEncoding.Base64 quirk.
//     mrsflow: EVAL ERROR (Encoding=0 not supported — kept strict).
//     What does PQ silently produce here?
Text.FromBinary(Text.ToBinary("hello", BinaryEncoding.Base64))

// q9: Binary.ToText Base64 of UTF-8 "hello" — canonical idiom.
//     mrsflow: "aGVsbG8="
Binary.ToText(Text.ToBinary("hello"), BinaryEncoding.Base64)

// q10: Csv.Document QuoteStyle.None preserves literal quotes.
//      mrsflow: 4 columns: "a", "\"b", "c\"", "d"
Csv.Document(
    Text.ToBinary("a,""b,c"",d"),
    [Delimiter=",", QuoteStyle=QuoteStyle.None])

// q11: Folder.Contents Attributes record on a known directory.
//      mrsflow (Linux): {Kind, Size, Hidden, Directory}.
//      On Windows PQ exposes more (ReadOnly, System, Archive, ...).
Folder.Contents("C:\Windows\System32"){0}[Attributes]

// q12: Excel.CurrentWorkbook from inside the host workbook.
//      mrsflow CLI w/o --param: empty Name/Content table.
//      In Excel: should include every named cell + ListObject.
Excel.CurrentWorkbook()

// q13: ODBC fold — column projection should push down to
//      `SELECT RITerritoryCode, RITerritoryDesc FROM RIGeographic`.
//      Semantics: 284 rows × 2 columns. Excel and mrsflow must match.
//      `[HierarchicalNavigation=true]` is required for Excel — without
//      it Excel returns a flat table list (no Database level) while
//      mrsflow keeps the nested shape; that divergence is a separate
//      follow-up (flat mode in mrsflow currently ignores the option).
//      Fold engagement is verified separately via the panicking
//      force_fn tests in mrsflow-core.
Table.SelectColumns(
    Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true])
        {[Name="NISAINT_CS",Kind="Database"]}[Data]
        {[Name="RIGeographic",Kind="Table"]}[Data],
    {"RITerritoryCode", "RITerritoryDesc"})

// q14: ODBC fold — row predicate should push down to
//      `SELECT * FROM RIGeographic WHERE RITerritoryCode = 'GB'`.
//      Semantics: GB territory subset only.
Table.SelectRows(
    Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true])
        {[Name="NISAINT_CS",Kind="Database"]}[Data]
        {[Name="RIGeographic",Kind="Table"]}[Data],
    each [RITerritoryCode] = "GB")

// q15: ODBC fold — combined projection + predicate.
//      Should push down to
//      `SELECT RITerritoryDesc FROM RIGeographic
//          WHERE RITerritoryCode = 'GB'`.
Table.SelectColumns(
    Table.SelectRows(
        Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true])
            {[Name="NISAINT_CS",Kind="Database"]}[Data]
            {[Name="RIGeographic",Kind="Table"]}[Data],
        each [RITerritoryCode] = "GB"),
    {"RITerritoryDesc"})
