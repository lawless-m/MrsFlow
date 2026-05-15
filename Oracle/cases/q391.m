let r = try Function.Invoke((x as number, y as number) => x + y, {3, 4}) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
