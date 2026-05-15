let r = try {
        Decimal.From("123.456"),
        Decimal.From("0.0001"),
        Decimal.From(null),
        Decimal.From(-99.99)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
