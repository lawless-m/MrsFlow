let
    r = try Table.Sort(
        #table({"k"}, {{"b"},{"A"},{"a"},{"C"}}),
        {{"k", Order.Ascending, Comparer.OrdinalIgnoreCase}})
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
