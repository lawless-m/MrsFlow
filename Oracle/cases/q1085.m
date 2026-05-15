// Json.FromValue round-trip: Value → JSON → back.
let r = try {
        Json.Document(Json.FromValue([a=1, b="hello"])),
        Json.Document(Json.FromValue({1, 2, 3})),
        Json.Document(Json.FromValue(null))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
