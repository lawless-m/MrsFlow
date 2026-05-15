let r = try
        let
            adder = (a) => (b) => a + b,
            add5 = adder(5),
            add10 = adder(10),
            applied = List.Transform({1, 2, 3}, add5)
        in
            applied
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
