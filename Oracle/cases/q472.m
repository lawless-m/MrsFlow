let r = try {
        Value.Is(Decimal.From(1.5), Decimal.Type),
        Value.Is(Decimal.From(1.5), type number),
        Value.Is(1.5, Decimal.Type),
        Value.Is(1.5, Double.Type)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
