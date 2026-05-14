let r = try Table.ColumnsOfType(
    Table.TransformColumnTypes(
        #table({"n","s"}, {{1,"x"}}),
        {{"n", Int64.Type}, {"s", type text}}),
    {type number})
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
