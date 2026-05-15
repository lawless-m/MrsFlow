let r = try Table.TransformColumnTypes(
        #table({"d"}, {{"2024-01-15"}, {"2024-06-30"}, {"2024-12-31"}}),
        {{"d", type date}}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
