// Table.NestedJoin null keys + LeftOuter.
let a = Table.FromRecords({[k=null, v="a1"], [k=1, v="a2"]}) in
let b = Table.FromRecords({[k=null, w="b1"], [k=1, w="b2"]}) in
let r = try {
        Table.NestedJoin(a, "k", b, "k", "nested", JoinKind.LeftOuter)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
