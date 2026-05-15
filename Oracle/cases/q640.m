let r = try {
        Table.IsEmpty(#table({"a"}, {})),
        Table.IsEmpty(#table({"a"}, {{1}})),
        Table.IsEmpty(#table({}, {}))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
