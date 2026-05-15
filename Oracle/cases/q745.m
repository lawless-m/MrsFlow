// Special values.
let r = try {
        try Number.FromText("Infinity") otherwise "err",
        try Number.FromText("-Infinity") otherwise "err",
        try Number.FromText("NaN") otherwise "err",
        try Number.FromText("inf") otherwise "err",
        Number.FromText("0"),
        Number.FromText("0.0"),
        Number.FromText("-0"),
        Number.FromText("-0.0"),
        try Number.FromText("null") otherwise "err",
        try Number.FromText(null) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
