let r = try
        let
            a = 12,
            b = 10,
            sum_via_bitwise = Number.BitwiseOr(Number.BitwiseAnd(a, b), Number.BitwiseXor(a, b))
        in
            sum_via_bitwise
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
