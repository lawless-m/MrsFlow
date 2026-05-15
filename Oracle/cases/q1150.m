// SelectFields against #shared narrows correctly.
let r = try {
        Record.FieldNames(Record.SelectFields(#shared, {"Text.From", "List.Sum"})),
        Record.FieldCount(Record.SelectFields(#shared, {"Text.From"})),
        Record.FieldCount(Record.RemoveFields(#shared, {"Text.From"})) = Record.FieldCount(#shared) - 1
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
