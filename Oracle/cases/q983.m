// Table.Join basic — Inner.
let a = Table.FromRecords({[k=1, v="a1"], [k=2, v="a2"], [k=3, v="a3"]}) in
let b = Table.FromRecords({[k=2, w="b2"], [k=3, w="b3"], [k=4, w="b4"]}) in
let r = try {
        Table.Join(a, "k", b, "k", JoinKind.Inner)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
