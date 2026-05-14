let r = try Decimal.From(0.1) + Decimal.From(0.2) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
