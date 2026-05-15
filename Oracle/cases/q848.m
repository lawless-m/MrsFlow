// Text.Length on unicode — does PQ count chars or UTF-16 code units?
let r = try {
        Text.Length(""),
        Text.Length("a"),
        Text.Length("café"),
        Text.Length("cafe#(0301)"),
        Text.Length("#(0001F600)"),
        Text.Length("a#(0001F600)b"),
        Text.Length("#(0001F600)#(0001F601)"),
        Text.Length("a#(0301)")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
