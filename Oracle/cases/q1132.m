// Table.ReplaceValue with Replacer.ReplaceValue: full-cell match.
let t = Table.FromRecords({[a=1, b="x"], [a=2, b="y"], [a=1, b="z"]}) in
let r = try {
        Table.ReplaceValue(t, 1, 99, Replacer.ReplaceValue, {"a"}),
        Table.ReplaceValue(t, "x", "X", Replacer.ReplaceValue, {"b"}),
        Table.ReplaceValue(t, null, 0, Replacer.ReplaceValue, {"a"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
