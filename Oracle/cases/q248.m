let r = try Splitter.SplitTextByCharacterTransition(
    {"0".."9"}, {"a".."z"})("123hello456world") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
