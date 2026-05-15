let r = try {
        Text.Lower("ÄÖÜß"),
        Text.Upper("äöüß"),
        Text.Lower("ÉÈÊË"),
        Text.Upper("éèêë")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
