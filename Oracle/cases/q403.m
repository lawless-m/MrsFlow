let r = try Expression.Evaluate("x + y", [x = 10, y = 32]) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
