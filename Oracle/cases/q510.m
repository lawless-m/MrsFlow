let r = try {
        try List.FirstN({1, 2, 3}, -1) otherwise "err",
        try List.LastN({1, 2, 3}, -1) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
