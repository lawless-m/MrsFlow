// Isolate Text.Lower("İ", "en-US") to see PQ output.
let r = try {
        Text.Lower("İ", "en-US"),
        Text.Lower("İ", "tr-TR"),
        Text.Lower("İ"),
        Text.Lower("İ", "en-US") = "i",
        Text.Lower("İ", "tr-TR") = "i"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
