let r = try Expression.Evaluate("let a = 5, b = 7 in a * b") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
