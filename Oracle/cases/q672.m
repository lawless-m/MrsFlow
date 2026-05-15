let r = try {
        try Number.IntegerDivide(1, 0) otherwise "err",
        try Number.IntegerDivide(-1, 0) otherwise "err",
        try Number.IntegerDivide(0, 0) otherwise "err",
        try Number.IntegerDivide(1.5, 0) otherwise "err",
        try Number.IntegerDivide(null, 5) otherwise "err",
        try Number.IntegerDivide(5, null) otherwise "err",
        try Number.IntegerDivide(null, null) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
