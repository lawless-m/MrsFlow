// List.Accumulate with empty-record seed — building up a record.
let r = try {
        List.Accumulate({"a", "b", "c"}, [], (acc, x) => Record.AddField(acc, x, Text.Upper(x))),
        List.Accumulate({1, 2, 3}, [count=0, sum=0], (acc, x) => [count=acc[count]+1, sum=acc[sum]+x])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
