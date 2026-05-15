// Table.Distinct with Comparer.OrdinalIgnoreCase as built-in arg.
let t = Table.FromRecords({
        [k=1, v="Apple"],
        [k=2, v="apple"],
        [k=3, v="BANANA"],
        [k=4, v="banana"]
    }) in
let r = try {
        Table.Distinct(t, {"v", Comparer.OrdinalIgnoreCase})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
