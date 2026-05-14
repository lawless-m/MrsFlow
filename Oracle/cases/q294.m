let r = try Table.ColumnsOfType(
    Table.TransformColumnTypes(
        #table({"n","s","b"}, {{1,"x",true}}),
        {{"n", Int64.Type}, {"s", type text}, {"b", type logical}}),
    {type number, type text})
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
