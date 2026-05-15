let r = try Table.AddIndexColumn(
        #table({"k"}, {{"x"}, {"y"}, {"z"}}),
        "idx",
        5,
        -1
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
