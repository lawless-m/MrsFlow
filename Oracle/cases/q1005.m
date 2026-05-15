// Table.ReplaceValue with multi-column target.
let t = Table.FromRecords({
        [a="A", b="A"],
        [a="B", b="A"],
        [a="A", b="C"]
    }) in
let r = try {
        Table.ReplaceValue(t, "A", "X", Replacer.ReplaceValue, {"a", "b"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
