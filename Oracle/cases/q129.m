let
    r = try List.Distinct({"a","A","b","B","c"},
        (x,y) => Value.Compare(Text.Lower(x), Text.Lower(y)))
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
