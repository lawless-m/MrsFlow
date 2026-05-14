let r = try Splitter.SplitTextByCharacterTransition(
    {"a".."z"}, {"0".."9"})("hello123world456") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
