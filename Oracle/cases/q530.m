let r = try
        let
            big = {1..100},
            window = List.Range(big, 45, 10)
        in
            window
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
