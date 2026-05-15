let r = try Table.TransformColumnTypes(
        #table({"a", "b", "c"}, {{"1", "true", "2024-01-01"}, {"2", "false", "2024-06-15"}}),
        {{"a", Int64.Type}, {"b", type logical}, {"c", type date}}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
