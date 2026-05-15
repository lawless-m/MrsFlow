// Thousand-separator handling.
let r = try {
        Number.FromText("1,000"),
        Number.FromText("1,234,567"),
        Number.FromText("1,234.5"),
        Number.FromText("-1,234.5"),
        try Number.FromText("1.234.567,89") otherwise "err",
        Number.FromText("1.234.567,89", "de-DE"),
        Number.FromText("1234,5", "de-DE"),
        Number.FromText("1 234 567,89", "fr-FR")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
