let r = try Table.FuzzyJoin(
    #table({"k"}, {{"apple"},{"banana"}}),
    "k",
    #table({"kr"}, {{"appel"}}),
    "kr",
    JoinKind.Inner,
    [SimilarityThreshold=0.8])
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
