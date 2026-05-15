let r = try
        let
            dt = #datetime(2024, 6, 15, 10, 30, 0),
            later = dt + #duration(0, 5, 30, 0)
        in
            later
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
