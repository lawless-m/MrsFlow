let r = try {
        Number.Mod(null, 5),
        Number.Mod(5, null),
        Number.IntegerDivide(null, 5),
        Number.IntegerDivide(5, null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
