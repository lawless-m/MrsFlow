let r = try
        let
            multiplyBy = (factor) => (x) => x * factor,
            triple = multiplyBy(3)
        in
            List.Transform({1, 2, 3, 4}, triple)
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
