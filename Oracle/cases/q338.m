let r = try Table.FuzzyGroup(
    #table({"k"}, {{"apple"},{"appel"},{"banana"}}),
    "k",
    {{"items", each _}})
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
