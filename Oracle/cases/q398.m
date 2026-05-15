let r = try Record.FieldNames(Type.FunctionParameters(type function (x as number, y as text) as logical)) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
