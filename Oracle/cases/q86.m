Table.Contains(
    #table({"k"}, {{"alpha"},{"beta"}}),
    [k="ALPHA"],
    (r,n) => Text.Lower(r[k]) = Text.Lower(n[k]))
