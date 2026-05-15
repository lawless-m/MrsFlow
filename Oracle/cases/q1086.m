// Json.Document malformed JSON.
let r = try {
        Json.Document("not valid json")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
