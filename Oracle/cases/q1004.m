// Table.ReplaceValue basic — replace a single value in a column.
let t = Table.FromRecords({
        [k=1, v="A"],
        [k=2, v="B"],
        [k=3, v="A"]
    }) in
let r = try {
        Table.ReplaceValue(t, "A", "X", Replacer.ReplaceValue, {"v"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
