// #shared scalar metadata.
let r = try {
        Record.FieldCount(#shared),
        Record.HasFields(#shared, "Text.From"),
        Record.HasFields(#shared, "List.Sum"),
        Record.HasFields(#shared, "Table.FromRecords"),
        Record.HasFields(#shared, "ThisDoesNotExist")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
