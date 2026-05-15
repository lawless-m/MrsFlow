let r = try {
        try List.AllTrue({true, null, true}) otherwise "err",
        try List.AnyTrue({null, false}) otherwise "err",
        try List.AllTrue({1, 2, 3}) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
