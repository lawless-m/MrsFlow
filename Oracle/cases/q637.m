let r = try {
        Table.First(#table({"v"}, {{1}, {2}, {3}})),
        Table.Last(#table({"v"}, {{1}, {2}, {3}})),
        Table.First(#table({"v"}, {})),
        Table.Last(#table({"v"}, {}))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
