let r = try {
        Type.Is(type number, type any),
        Type.Is(type text, type number),
        Type.Is(type number, type number),
        Type.Is(type {number}, type list)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
