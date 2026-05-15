let r = try Table.Column(
        #table({"a", "b", "c"}, {{1, 2, 3}, {4, 5, 6}, {7, 8, 9}}),
        "b"
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
