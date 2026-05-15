let r = try Table.SelectRows(
        #table({"name", "score"}, {{"Alice", 85}, {"Bob", 72}, {"Charlie", 91}, {"Dave", 67}}),
        each Text.StartsWith([name], "A") or [score] > 80
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
