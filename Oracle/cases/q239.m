let r = try Csv.Document(
    Binary.Combine({#binary({0xEF,0xBB,0xBF}), Text.ToBinary("a,b#(lf)1,2")})) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
