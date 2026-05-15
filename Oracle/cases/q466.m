let r = try {
        Currency.From(123.45),
        Currency.From(0),
        Currency.From(null),
        Currency.From(-5.99)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
