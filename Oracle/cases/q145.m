let
    r = try Table.Sort(
        #table({"k"}, {{"b"},{"A"},{"a"},{"C"}}),
        (r1,r2) => Value.Compare(Text.Lower(r1[k]), Text.Lower(r2[k])))
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
