let r = try
        let
            joined = Table.NestedJoin(
                #table({"k"}, {{"a"}, {"b"}, {"c"}}),
                "k",
                #table({"k", "w"}, {{"a", 10}, {"a", 20}, {"b", 30}, {"d", 40}}),
                "k",
                "Sub",
                JoinKind.LeftOuter
            ),
            rowCounts = Table.AddColumn(joined, "n", each Table.RowCount([Sub]))
        in
            Table.SelectColumns(rowCounts, {"k", "n"})
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
