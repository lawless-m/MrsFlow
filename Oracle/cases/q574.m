let r = try
        let
            pairs = {{1, 2}, {3, 4}, {5, 6}}
        in
            List.Transform(pairs, each _{0} + _{1})
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
