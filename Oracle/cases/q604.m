let r = try Table.ReplaceValue(
        #table({"v"}, {{null}, {"x"}, {null}, {"y"}}),
        null,
        "MISSING",
        Replacer.ReplaceValue,
        {"v"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
