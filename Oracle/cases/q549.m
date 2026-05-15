let r = try {
        try Number.Asin(2) otherwise "err",
        try Number.Acos(-2) otherwise "err",
        try Number.Tan(1.5707963267948966) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
