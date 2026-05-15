let r = try Table.FuzzyNestedJoin(
    #table({"k"}, {{"apple"}}),
    {"k"},
    #table({"kr"}, {{"appel"}}),
    {"kr"},
    "right")
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
