// Json.Document basic — scalars and simple objects/arrays.
let r = try {
        Json.Document("42"),
        Json.Document("3.14"),
        Json.Document("""hello"""),
        Json.Document("true"),
        Json.Document("null"),
        Json.Document("[]"),
        Json.Document("{}")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
