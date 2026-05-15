let r = try Table.ReplaceValue(
        #table({"a", "b"}, {{1, 1}, {1, 2}, {2, 1}}),
        1,
        99,
        Replacer.ReplaceValue,
        {"a"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
