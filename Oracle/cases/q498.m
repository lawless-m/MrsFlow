let r = try
        let
            samples = List.Transform({1..5}, each Text.NewGuid()),
            distinct = List.Distinct(samples)
        in
            List.Count(distinct)
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
