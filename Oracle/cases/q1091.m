// Csv.Document ragged rows — shorter row, longer row, empty row.
let csv = "a,b,c#(cr,lf)1,2#(cr,lf)4,5,6,7#(cr,lf)#(cr,lf)8,9,10" in
let r = try {
        Csv.Document(Text.ToBinary(csv, TextEncoding.Utf8))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
