// Table.Join with empty side(s).
let nonEmpty = Table.FromRecords({[k=1, v="a"]}) in
let empty = Table.FromRecords({}) in
let r = try {
        Table.Join(nonEmpty, "k", empty, "k", JoinKind.LeftOuter),
        Table.Join(empty, "k", nonEmpty, "k", JoinKind.LeftOuter),
        Table.Join(empty, "k", empty, "k", JoinKind.Inner)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
