// Json.Document unicode \u escapes.
let r = try {
        Json.Document("""é"""),
        Json.Document("""→"""),
        Json.Document("""F600""")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
