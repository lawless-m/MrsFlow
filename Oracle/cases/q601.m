let r = try Table.ReplaceValue(
        #table({"v"}, {{"hello"}, {"world"}, {"hello"}}),
        "hello",
        "HI",
        Replacer.ReplaceValue,
        {"v"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
