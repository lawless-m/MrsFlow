Table.Distinct(
    #table({"k","v"}, {{"a",1},{"A",2},{"b",3}}),
    (r1,r2) => Text.Lower(r1[k]) = Text.Lower(r2[k]))
