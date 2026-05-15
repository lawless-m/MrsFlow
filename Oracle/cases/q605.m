let r = try Table.ReplaceErrorValues(
        Table.AddColumn(
            #table({"n"}, {{2}, {0}, {4}, {0}, {8}}),
            "inv",
            each if [n] = 0 then error "div by zero" else 100 / [n]
        ),
        {{"inv", -1}}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
