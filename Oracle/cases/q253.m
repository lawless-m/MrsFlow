let r = try Json.Document("[[[1,2],[3,4]],[[5,6]]]") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
