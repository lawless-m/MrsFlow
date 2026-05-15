let r = try {
        Int64.From(123.7),
        Int64.From(123.4),
        Int64.From(-123.7),
        Int64.From("42")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
