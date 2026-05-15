let r = try Splitter.SplitTextByLengths({2, 3, 1})("abcdefg") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
