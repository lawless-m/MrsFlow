let
    r = try List.Difference({"A","B","C"}, {"a","c"},
        (x,y) => Text.Lower(x) = Text.Lower(y))
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
