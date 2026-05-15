let r = try Function.Invoke(Text.Combine, {{"a", "b", "c"}, "-"}) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
