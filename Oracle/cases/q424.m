let r = try
        let
            joined = Table.NestedJoin(
                #table({"k", "v"}, {{"a", 1}, {"b", 2}}),
                "k",
                #table({"k", "w"}, {{"a", 10}, {"a", 20}, {"b", 30}}),
                "k",
                "Sub",
                JoinKind.LeftOuter
            )
        in
            Table.ColumnNames(joined)
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
