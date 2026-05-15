let r = try Table.SplitColumn(
        #table({"full"}, {{"a,b,c"}, {"d,e"}, {"f"}}),
        "full",
        Splitter.SplitTextByDelimiter(","),
        {"p1", "p2", "p3"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
