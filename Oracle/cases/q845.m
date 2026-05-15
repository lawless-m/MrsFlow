// Text.Reverse with supplementary-plane chars (emoji, U+1F600 etc).
// In .NET these are surrogate pairs — reversing UTF-16 code units splits
// them. Reversing codepoints/chars keeps them intact.
let r = try {
        Text.Reverse("a#(0001F600)b"),
        Text.Length("a#(0001F600)b"),
        Text.Reverse("#(0001F600)#(0001F601)")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
