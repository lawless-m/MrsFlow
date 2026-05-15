let r = try {
        try Character.FromNumber(-1) otherwise "err",
        try Character.FromNumber(1114112) otherwise "err",
        try Character.ToNumber("") otherwise "err",
        try Character.ToNumber("ab") otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
