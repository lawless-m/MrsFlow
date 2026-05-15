let r = try Table.Sort(
        #table({"n"}, {{3}, {1}, {2}, {5}, {4}}),
        {"n", Order.Descending}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
