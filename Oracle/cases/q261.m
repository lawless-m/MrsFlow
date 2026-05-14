let r = try Table.Unpivot(
    #table({"id","jan","feb","mar"}, {{"a",1,2,3}}),
    {"jan","feb","mar"}, "month", "value") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
