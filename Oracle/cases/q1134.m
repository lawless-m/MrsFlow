// Table.ReplaceValue with null cells in the target column.
let t = Table.FromRecords({[a=1], [a=null], [a=2], [a=null]}) in
let r = try {
        Table.ReplaceValue(t, null, 0, Replacer.ReplaceValue, {"a"}),
        Table.ReplaceValue(t, 1, null, Replacer.ReplaceValue, {"a"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
