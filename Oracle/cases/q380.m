let r = try Combiner.CombineTextByPositions({0, 5, 10})({"abc", "defg", "hi"}) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
