// Table.Join with duplicate keys — many-to-many.
let a = Table.FromRecords({[k=1, v="a1a"], [k=1, v="a1b"]}) in
let b = Table.FromRecords({[k=1, w="b1a"], [k=1, w="b1b"]}) in
let r = try {
        Table.Join(a, "k", b, "k", JoinKind.Inner)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
