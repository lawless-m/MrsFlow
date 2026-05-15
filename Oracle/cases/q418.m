let r = try Table.Group(
        #table({"k", "v"}, {{"a", 1}, {"a", 2}, {"b", 3}}),
        {"k"},
        {{"Values", each [v], type list}}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
