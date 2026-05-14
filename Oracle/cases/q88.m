List.Difference({"A","B","C"}, {"a","c"},
    (x,y) => Text.Lower(x) = Text.Lower(y))
