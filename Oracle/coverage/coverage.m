// coverage.m — coverage dashboard (M side). Runs in both PQ and mrsflow.
//
// Output: a table with one row per name in (Excel #shared ∪ mrsflow #shared).
// Columns emitted from M:
//   Name        - the #shared field name
//   InPQ        - true iff Excel's #shared exposes it
//   InMrsflow   - true iff mrsflow's #shared exposes it
//   Kind        - "function" / "constant" / "missing"
//                 (introspected from THIS engine's #shared)
//
// The OracleCases and OracleStatus columns are joined in by the renderer
// (Oracle/coverage/render.ps1) from the pre-generated TSVs. Doing that
// in M was attempted but tripped a closure-capture issue in mrsflow's
// evaluator — punting the lookup to PowerShell keeps the M side simple
// and engine-agnostic.

let
    casesDir = "cases",

    // --- Read both engines' #shared dumps ---
    // `excludeNames` strips harness leakage: `EvalFile` is the workbook
    // wrapper-query name that Excel injects into `#shared` for the
    // catalog itself; mrsflow doesn't surface it (mrsflow runs the .m
    // file directly). The space-filter further drops a junk row
    // "Invoked FunctionEvalFile" that the workbook export concatenates
    // from the "Invoked Function" header and the EvalFile name.
    excludeNames = {"EvalFile"},
    readNameList = (relPath) =>
        let
            raw   = try Text.FromBinary(File.Contents(relPath), TextEncoding.Utf8)
                      otherwise "",
            lines = Lines.FromText(raw),
            kept  = List.Select(lines, each
                Text.Length(Text.Trim(_)) > 0
                and not Text.Contains(_, " ")
                and not List.Contains(excludeNames, _))
        in
            List.Distinct(kept),

    excelSet   = readNameList(casesDir & "/q1165.excel.out"),
    mrsflowSet = readNameList(casesDir & "/q1165.mrsflow.out"),

    selfNames = Record.FieldNames(#shared),
    allNames = List.Sort(List.Distinct(excelSet & mrsflowSet)),

    rows = List.Transform(allNames, (name) =>
        let
            inPQ      = List.Contains(excelSet, name),
            inMrsflow = List.Contains(mrsflowSet, name),
            kind =
                if not List.Contains(selfNames, name) then "missing"
                else
                    let
                        v = try Record.Field(#shared, name) otherwise null,
                        isFn = try Value.Is(v, type function) otherwise false
                    in
                        if isFn then "function" else "constant"
        in
            { name, inPQ, inMrsflow, kind }),

    result = Table.FromRows(
        rows,
        type table [
            Name       = text,
            InPQ       = logical,
            InMrsflow  = logical,
            Kind       = text
        ])
in
    result
