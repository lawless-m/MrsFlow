let r = try
        let
            t = #table({"Column1", "Column2"}, {{"a", "b"}, {"c", "d"}, {"e", "f"}}),
            promoted = Table.PromoteHeaders(t)
        in
            Table.ColumnNames(promoted)
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
