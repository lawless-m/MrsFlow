let r = try {
        Table.Min(#table({"v"}, {{3}, {1}, {2}}), "v"),
        Table.Max(#table({"v"}, {{3}, {1}, {2}}), "v")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
