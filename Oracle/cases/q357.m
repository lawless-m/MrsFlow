let r = try {Number.Round(3.14159, 2), Number.Round(3.14159, 3), Number.Round(3.14159, 0), Number.Round(123.456, -1)} in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
