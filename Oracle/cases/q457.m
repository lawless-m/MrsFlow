let r = try {
        Logical.FromText("true"),
        Logical.FromText("false"),
        Logical.FromText("TRUE"),
        Logical.FromText("False")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
