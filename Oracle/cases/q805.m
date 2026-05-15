// Text.Lower/Upper default culture — ß stays ß under .NET legacy Upper.
let r = try {
        Text.Upper("hello"),
        Text.Upper("café"),
        Text.Upper("straße"),
        Text.Upper("ß"),
        Text.Lower("HELLO"),
        Text.Lower("CAFÉ"),
        Text.Lower("STRASSE")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
