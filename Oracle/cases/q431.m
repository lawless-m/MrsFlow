let r = try Table.TransformColumnTypes(
        #table({"n", "t"}, {{"1", "a"}, {"2", "b"}, {"3", "c"}}),
        {{"n", Int64.Type}}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
