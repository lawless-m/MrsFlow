let r = try {
        try List.Range({1, 2, 3, 4, 5}, 10, 3) otherwise "err",
        try List.Range({1, 2, 3, 4, 5}, 0, 100) otherwise "err",
        try List.Range({1, 2, 3, 4, 5}, -1, 2) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
