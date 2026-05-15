let r = try {
        Number.Asin(0),
        Number.Asin(1),
        Number.Acos(1),
        Number.Acos(0),
        Number.Atan(0),
        Number.Atan(1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
