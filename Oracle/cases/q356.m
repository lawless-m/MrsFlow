let r = try {Number.Round(0.5), Number.Round(1.5), Number.Round(2.5), Number.Round(-0.5), Number.Round(-1.5)} in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
