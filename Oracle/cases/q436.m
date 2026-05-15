let r = try Table.SplitColumn(
        #table({"full"}, {{"a,b"}, {"c,d"}, {"e,f"}}),
        "full",
        Splitter.SplitTextByDelimiter(","),
        {"first", "second"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
