Table.AddColumn(
    #table({"A"}, {{10}}),
    "B",
    each [A] * 2)
