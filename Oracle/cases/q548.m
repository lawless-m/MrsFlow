let r = try {
        Number.Atan2(1, 1),
        Number.Atan2(1, 0),
        Number.Atan2(0, -1),
        Number.Atan2(-1, -1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
