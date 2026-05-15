// Table.ReplaceValue with Replacer.ReplaceText (substring replacement).
let t = Table.FromRecords({
        [v="hello world"],
        [v="say hello"],
        [v="no match"]
    }) in
let r = try {
        Table.ReplaceValue(t, "hello", "HI", Replacer.ReplaceText, {"v"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
