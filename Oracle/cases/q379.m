let r = try Combiner.CombineTextByLengths({2, 3, 1})({"ab", "cde", "f"}) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
