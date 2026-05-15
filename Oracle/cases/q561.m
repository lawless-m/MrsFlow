let r = try {
        Text.Proper("hello world"),
        Text.Proper("HELLO WORLD"),
        Text.Proper("hELLo wORLD"),
        Text.Proper("a")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
