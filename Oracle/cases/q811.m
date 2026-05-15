// Idempotent under repeated application; cross-culture comparison.
let r = try {
        Text.Upper(Text.Upper("hello")) = Text.Upper("hello"),
        Text.Lower(Text.Lower("HELLO")) = Text.Lower("HELLO"),
        Text.Lower(Text.Upper("hello")) = "hello",
        Text.Upper("istanbul", "tr-TR") <> Text.Upper("istanbul", "en-US"),
        Text.Lower("İ", "tr-TR") <> Text.Lower("İ", "en-US")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
