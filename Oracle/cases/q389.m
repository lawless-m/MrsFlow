let r = try {
        Value.Compare(null, 1),
        Value.Compare(1, null),
        Value.Compare(null, null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
