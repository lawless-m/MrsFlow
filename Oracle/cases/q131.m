let
    r = try List.Intersect({{"A","B","C"}, {"a","b"}},
        (x,y) => Text.Lower(x) = Text.Lower(y))
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
