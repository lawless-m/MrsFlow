let r = try {Number.RoundDown(2.9), Number.RoundDown(-2.9), Number.RoundDown(2.1), Number.RoundDown(-2.1), Number.RoundDown(0)} in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
