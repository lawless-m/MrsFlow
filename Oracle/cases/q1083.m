// Json.Document escape sequences in strings.
// Build the JSON text via #(NNNN) M escapes to keep backslashes out of
// the M lexer.
let bs = "#(005c)" in
let r = try {
        Json.Document("""" & bs & "t" & bs & "n" & bs & "r" & bs & "b" & bs & "f"""),
        Json.Document("""é"""),
        Json.Document("""→""")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
