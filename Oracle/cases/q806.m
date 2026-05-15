// Text.Lower/Upper tr-TR — I↔ı and İ↔i pairs.
let r = try {
        Text.Upper("istanbul", "tr-TR"),
        Text.Upper("ı", "tr-TR"),
        Text.Upper("i", "tr-TR"),
        Text.Lower("İSTANBUL", "tr-TR"),
        Text.Lower("İ", "tr-TR"),
        Text.Lower("I", "tr-TR")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
