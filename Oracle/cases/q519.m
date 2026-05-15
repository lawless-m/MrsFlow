let r = try
        let
            nums = {1, 2, 3, 4, 5},
            checks = List.Transform(nums, each _ > 0)
        in
            List.AllTrue(checks)
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
