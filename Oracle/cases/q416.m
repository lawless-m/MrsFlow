let r = try Table.Group(
        #table({"k", "v"}, {{"a", 1}, {"a", 2}, {"b", 3}, {"b", 4}, {"a", 5}}),
        {"k"},
        {{"Sum", each List.Sum([v]), Int64.Type}}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
