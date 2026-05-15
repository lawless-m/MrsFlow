let r = try {
        Number.BitwiseOr(12, 10),
        Number.BitwiseOr(0, 255),
        Number.BitwiseOr(240, 15)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
