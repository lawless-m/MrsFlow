let r = try {
        try Logical.FromText("1") otherwise "err",
        try Logical.FromText("0") otherwise "err",
        try Logical.FromText("yes") otherwise "err",
        try Logical.FromText("") otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
