let
    r = try Table.Distinct(
        #table({"k","v"}, {{"a",1},{"A",2},{"b",3}}),
        {"k", Comparer.OrdinalIgnoreCase})
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
