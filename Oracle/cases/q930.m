// List.Transform with 0-arg or 3-arg function.
let r = try {
        List.Transform({1, 2, 3}, () => 99),
        List.Transform({1, 2, 3}, (a, b, c) => a + b + c)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
