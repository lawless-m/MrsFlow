let r = try {
        Logical.From(true),
        Logical.From(false),
        Logical.From(1),
        Logical.From(0),
        Logical.From(null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
