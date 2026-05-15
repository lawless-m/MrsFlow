let r = try Table.ReplaceValue(
        #table({"v"}, {{"foo bar"}, {"bar baz"}, {"qux"}}),
        "bar",
        "X",
        Replacer.ReplaceText,
        {"v"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
