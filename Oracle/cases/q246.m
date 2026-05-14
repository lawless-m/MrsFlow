let r = try Splitter.SplitTextByCharacterTransition(
    {"a","b","c"}, {"0","1","2","3","4","5","6","7","8","9"})("abc123") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
