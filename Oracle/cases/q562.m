let r = try {
        Text.Trim("  hello  "),
        Text.Trim("hello"),
        Text.Trim("   "),
        Text.Trim("#(tab)hello#(lf)")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
