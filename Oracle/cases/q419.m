let r = try Table.Group(
        #table({"k", "v"}, {}),
        {"k"},
        {{"Count", each Table.RowCount(_), Int64.Type}}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
