let r = try Table.SplitColumn(
        #table({"csv"}, {{"a,""b,c"",d"}, {"e,""f,g"",h"}}),
        "csv",
        Splitter.SplitTextByDelimiter(",", QuoteStyle.Csv),
        {"p1", "p2", "p3"}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
