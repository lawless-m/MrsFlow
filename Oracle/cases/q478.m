let r = try
        let
            samples = List.Transform({1..10}, each Number.Random()),
            allInRange = List.AllTrue(List.Transform(samples, each _ >= 0 and _ < 1))
        in
            allInRange
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
