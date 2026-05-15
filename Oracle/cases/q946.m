// Table.Sort with comparer lambda per column.
let t = Table.FromRecords({
        [name="Apple"],
        [name="banana"],
        [name="Cherry"]
    }) in
let r = try {
        Table.Sort(t, "name"),
        // Lambda comparer for case-insensitive sort.
        Table.Sort(t, {"name", (a, b) => Value.Compare(Text.Lower(a), Text.Lower(b))})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
