let r = try {
        try List.Repeat({1}, -1) otherwise "err",
        List.Repeat({"a", "b"}, 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
