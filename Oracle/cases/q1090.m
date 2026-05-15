// Csv.Document with QuoteStyle.None — quotes are literal.
let csv = "a,b#(cr,lf)""quoted"",unquoted#(cr,lf)" in
let r = try {
        Csv.Document(Text.ToBinary(csv, TextEncoding.Utf8), [QuoteStyle = QuoteStyle.None]),
        Csv.Document(Text.ToBinary(csv, TextEncoding.Utf8), [QuoteStyle = QuoteStyle.Csv])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
