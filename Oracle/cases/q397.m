let r = try Record.FieldNames(Type.RecordFields(type [a = number, b = text, c = logical])) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
