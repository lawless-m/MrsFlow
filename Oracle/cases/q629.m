let r = try Table.PromoteHeaders(
        #table({"Column1"}, {})
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=Table.ColumnNames(r[Value])]
