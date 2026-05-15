// Csv.Document custom delimiter.
let tsv = "a#(tab)b#(tab)c#(cr,lf)1#(tab)2#(tab)3" in
let pipe = "a|b#(cr,lf)X|Y" in
let r = try {
        Csv.Document(Text.ToBinary(tsv, TextEncoding.Utf8), [Delimiter = "#(tab)"]),
        Csv.Document(Text.ToBinary(pipe, TextEncoding.Utf8), [Delimiter = "|"])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
