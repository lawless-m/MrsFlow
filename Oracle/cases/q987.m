// Table.Join with null keys — PQ behaviour: null doesn't match null.
let a = Table.FromRecords({[k=null, v="a-null"], [k=1, v="a1"]}) in
let b = Table.FromRecords({[k=null, w="b-null"], [k=1, w="b1"]}) in
let r = try {
        Table.Join(a, "k", b, "k", JoinKind.Inner),
        Table.Join(a, "k", b, "k", JoinKind.LeftOuter)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
