// Access stub-only names via #shared and verify they're callable values
// (functions or types), not garbage.
let r = try {
        // These names are in PQ_NAMES_STUB padding; just verify they ARE
        // present as fields and are functions/values, not text noise.
        Value.Is(Record.Field(#shared, "OData.Feed"), type function),
        Value.Is(Record.Field(#shared, "Web.Contents"), type function),
        // Constants we know exist.
        Record.Field(#shared, "JoinKind.Inner") <> null
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
