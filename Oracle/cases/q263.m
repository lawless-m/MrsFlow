let r = try Table.Pivot(
    #table({"id","month","value"},
        {{"a","jan",1},{"a","feb",2},{"a","mar",3}}),
    {"jan","feb","mar"}, "month", "value") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
