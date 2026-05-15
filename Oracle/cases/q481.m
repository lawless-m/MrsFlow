let r = try {
        try Number.Mod(10, 0) otherwise "err",
        try Number.IntegerDivide(10, 0) otherwise "err",
        try Number.Mod(0, 5) otherwise "err",
        try Number.IntegerDivide(0, 5) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
