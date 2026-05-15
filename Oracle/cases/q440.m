let r = try Table.SplitColumn(
        #table({"by_pos"}, {{"abcdef"}, {"123456"}}),
        "by_pos",
        Splitter.SplitTextByLengths({2, 2, 2}),
        {"p1", "p2", "p3"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
