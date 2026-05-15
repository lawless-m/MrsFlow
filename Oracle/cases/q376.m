let r = try Combiner.CombineTextByDelimiter(",")({"a", "b", "c", "d"}) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
