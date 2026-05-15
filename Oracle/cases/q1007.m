// Table.ReplaceValue with null target — replace nulls.
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
            else [HasError=false, Value=r[Value]]
