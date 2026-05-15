let r = try Table.TransformColumnTypes(
        #table({"n"}, {{"1.234,56"}, {"2.345,67"}}),
        {{"n", type number}},
        "de-DE"
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
