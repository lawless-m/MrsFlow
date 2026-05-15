let r = try
        let
            a = 5,
            b = 10,
            checks = {a > 0, b > 0, a < b}
        in
            {List.AllTrue(checks), List.AnyTrue(checks)}
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
