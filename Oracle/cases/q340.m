let r = try Table.FuzzyJoin(
    #table({"k"}, {{"apple"}}),
    "k",
    #table({"kr"}, {{"apple"}}),
    "kr",
    JoinKind.Inner,
    [SimilarityThreshold=1.0])
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
