let r = try {
        Percentage.From("50%"),
        Percentage.From("100%"),
        Percentage.From("0%"),
        Percentage.From("12.5%"),
        Percentage.From("-25%")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
