let r = try Table.Pivot(
    #table({"id","month","value"},
        {{"a","jan",1},{"a","jan",10},{"a","feb",2}}),
    {"jan","feb"}, "month", "value", List.Sum) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
