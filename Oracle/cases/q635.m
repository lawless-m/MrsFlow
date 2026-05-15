let r = try {
        try Table.RemoveColumns(#table({"a", "b"}, {{1, 2}}), {"x"}) otherwise "err",
        try Table.SelectColumns(#table({"a", "b"}, {{1, 2}}), {"x"}) otherwise "err",
        try Table.Column(#table({"a"}, {{1}}), "x") otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
