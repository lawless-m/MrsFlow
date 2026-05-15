let r = try Table.FuzzyJoin(
    #table({"k"}, {{"apple"},{"banana"}}),
    "k",
    #table({"kr"}, {{"appel"},{"bananna"}}),
    "kr")
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
