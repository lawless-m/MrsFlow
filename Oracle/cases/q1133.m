// Table.ReplaceValue with Replacer.ReplaceText: substring-in-cell.
let t = Table.FromRecords({[a=1, b="hello world"], [a=2, b="goodbye world"], [a=3, b="other"]}) in
let r = try {
        Table.ReplaceValue(t, "world", "M", Replacer.ReplaceText, {"b"}),
        Table.ReplaceValue(t, "o", "0", Replacer.ReplaceText, {"b"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
