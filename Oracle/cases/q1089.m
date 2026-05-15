// Csv.Document with embedded quotes (RFC4180 — "" escapes ").
// Build via concat to keep the literal lexer-clean.
let q = """" in
let crlf = "#(cr,lf)" in
let csv = "a,b" & crlf & q & "hello, world" & q & "," & q & "say " & q & q & "hi" & q & q & q & crlf in
let r = try {
        Csv.Document(Text.ToBinary(csv, TextEncoding.Utf8))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
