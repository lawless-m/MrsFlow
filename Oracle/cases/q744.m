// Invalid inputs.
let r = try {
        try Number.FromText("abc") otherwise "err",
        try Number.FromText("12abc") otherwise "err",
        try Number.FromText("1..2") otherwise "err",
        try Number.FromText("1.2.3") otherwise "err",
        try Number.FromText("--1") otherwise "err",
        try Number.FromText("+-1") otherwise "err",
        try Number.FromText("1e") otherwise "err",
        try Number.FromText("e5") otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
