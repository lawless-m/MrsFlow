let
    a = #table({"k","x"}, {{1,"hello"}}),
    b = #table({"k","y"}, {{1,"world"}}),
    j = Table.NestedJoin(a, {"k"}, b, {"k"}, "right", JoinKind.LeftOuter)
in
    Table.ExpandTableColumn(j, "right", {"y"})
