let r = try Table.Group(
        #table({"region", "category", "sales"}, {{"N", "X", 10}, {"N", "Y", 20}, {"S", "X", 30}, {"N", "X", 40}}),
        {"region", "category"},
        {{"Total", each List.Sum([sales]), Int64.Type}}
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
