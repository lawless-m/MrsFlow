let r = try {
        Logical.From(2),
        Logical.From(-1),
        Logical.From(0.5),
        try Logical.From("true") otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
