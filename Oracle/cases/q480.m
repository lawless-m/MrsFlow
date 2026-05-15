let r = try
        let
            samples = List.Transform({1..20}, each Number.RandomBetween(0, 100)),
            distinctCount = List.Count(List.Distinct(samples))
        in
            distinctCount > 1
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
