let r = try Combiner.CombineTextByDelimiter(",", QuoteStyle.Csv)({"a", "b,c", "d""e"}) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
