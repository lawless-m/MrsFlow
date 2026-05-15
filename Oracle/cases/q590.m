let r = try Table.Sort(
        #table({"a", "b"}, {{1, 1}, {1, 2}, {1, 1}, {2, 1}, {1, 2}}),
        "a"
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
