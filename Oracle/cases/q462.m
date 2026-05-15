let r = try {
        Percentage.From(0.5),
        Percentage.From(1),
        Percentage.From(null),
        Percentage.From(true)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
