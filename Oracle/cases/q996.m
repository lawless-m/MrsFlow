// Table.NestedJoin RightOuter / FullOuter.
let a = Table.FromRecords({[k=1, v="a1"], [k=2, v="a2"]}) in
let b = Table.FromRecords({[k=2, w="b2"], [k=3, w="b3"]}) in
let r = try {
        Table.NestedJoin(a, "k", b, "k", "nested", JoinKind.RightOuter),
        Table.NestedJoin(a, "k", b, "k", "nested", JoinKind.FullOuter)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
