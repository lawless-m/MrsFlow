let r = try
        let
            dates = List.Dates(#date(2024, 6, 15), 5, #duration(1, 0, 0, 0))
        in
            dates
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
