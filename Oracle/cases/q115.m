Table.Group(
    #table({"k","v"}, {{"a",1},{"a",2},{"b",3},{"a",4}}),
    "k",
    {{"total", each List.Sum([v])}},
    GroupKind.Local)
