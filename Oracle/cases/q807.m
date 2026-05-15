// Text.Lower/Upper az-AZ — same I↔ı/İ↔i Turkic mapping.
let r = try {
        Text.Upper("istanbul", "az-AZ"),
        Text.Upper("ı", "az-AZ"),
        Text.Lower("İSTANBUL", "az-AZ"),
        Text.Lower("I", "az-AZ")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
