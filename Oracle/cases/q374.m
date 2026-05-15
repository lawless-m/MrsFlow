let r = try Splitter.SplitTextByEachDelimiter({",", ";", "|"})("a,b;c|d,e") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
