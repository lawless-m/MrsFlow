// Table.ReplaceValue with no match — table unchanged.
let t = Table.FromRecords({[v="A"], [v="B"]}) in
let r = try {
        Table.ReplaceValue(t, "Z", "X", Replacer.ReplaceValue, {"v"}) = t,
        Table.ReplaceValue(t, "A", "A", Replacer.ReplaceValue, {"v"}) = t
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
