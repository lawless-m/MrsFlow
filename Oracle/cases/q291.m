let r = try Table.Schema(
    Table.TransformColumnTypes(
        #table({"n","s","b"}, {{1,"x",true}}),
        {{"n", Int64.Type}, {"s", type text}, {"b", type logical}}))
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
