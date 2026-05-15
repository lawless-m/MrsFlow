// Currency symbols and parentheses (en-US negative form).
let r = try {
        try Number.FromText("$100") otherwise "err",
        try Number.FromText("$100.50") otherwise "err",
        try Number.FromText("£100") otherwise "err",
        try Number.FromText("€100") otherwise "err",
        try Number.FromText("(100)") otherwise "err",
        try Number.FromText("(100.50)") otherwise "err",
        try Number.FromText("($100)") otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
