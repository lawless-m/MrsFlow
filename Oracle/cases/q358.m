let r = try {Number.RoundUp(2.1), Number.RoundUp(-2.1), Number.RoundUp(2.9), Number.RoundUp(-2.9), Number.RoundUp(0)} in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
