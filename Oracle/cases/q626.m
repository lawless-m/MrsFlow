let r = try Table.PromoteHeaders(
        #table({"Column1", "Column2", "Column3"}, {{"a", "b", "c"}, {"1", "2", "3"}})
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
