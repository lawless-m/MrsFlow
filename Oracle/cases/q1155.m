// JSON round-trip: serialise then re-parse the boundary numbers.
let original = {999999999999999, 1000000000000000, 9007199254740992} in
let text = Text.FromBinary(Json.FromValue(original), TextEncoding.Utf8) in
let parsed = Json.Document(text) in
let r = try {
        text,
        parsed,
        original = parsed
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
