let r = try Table.UnpivotOtherColumns(
    #table({"id","jan","feb"}, {{"a",1,2}, {"b",3,4}}),
    {"id"}, "month", "value") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
