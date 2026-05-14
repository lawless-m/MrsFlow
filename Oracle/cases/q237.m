let r = try Csv.Document(
    Text.ToBinary("a,b#(lf)1,2"),
    [Encoding=65001]) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
