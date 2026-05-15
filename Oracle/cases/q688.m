// Number.Log with base (third arg).
let r = try {
        Number.Log(8, 2),
        Number.Log(100, 10),
        Number.Log(1, 10),
        Number.Log(2, 2),
        Number.Log(0.5, 2),
        try Number.Log(0, 2) otherwise "err",
        try Number.Log(-1, 2) otherwise "err",
        try Number.Log(8, 1) otherwise "err",
        try Number.Log(8, 0) otherwise "err",
        try Number.Log(8, -2) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
