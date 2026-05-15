// Thousands grouping #,##0.00.
let r = try {
        Number.ToText(1234.5, "#,##0.00"),
        Number.ToText(1234567.89, "#,##0.00"),
        Number.ToText(0, "#,##0.00"),
        Number.ToText(-1234.5, "#,##0.00"),
        Number.ToText(1234567, "#,##0"),
        Number.ToText(999, "#,##0"),
        Number.ToText(1234.5, "#,##0.00", "de-DE"),
        Number.ToText(1234.5, "#,##0.00", "fr-FR")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
