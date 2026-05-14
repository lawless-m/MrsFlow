let r = try Table.ReplaceValue(
    #table({"v"}, {{1},{2},{1}}),
    1, 99, Replacer.ReplaceValue, {"v"}) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
