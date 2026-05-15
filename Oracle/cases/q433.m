let r = try Table.TransformColumnTypes(
        #table({"n"}, {{"1.5"}, {"2.7"}, {"3.14"}}),
        {{"n", type number}}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
