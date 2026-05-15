let r = try {
        try Number.Mod(1, 0) otherwise "err",
        try Number.Mod(-1, 0) otherwise "err",
        try Number.Mod(0, 0) otherwise "err",
        try Number.Mod(1.5, 0) otherwise "err",
        try Number.Mod(null, 5) otherwise "err",
        try Number.Mod(5, null) otherwise "err",
        try Number.Mod(null, null) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
