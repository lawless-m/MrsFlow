let r = try {
        Number.BitwiseXor(12, 10),
        Number.BitwiseXor(255, 255),
        Number.BitwiseXor(0, 255)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
