List.Intersect({{"A","B","C"}, {"a","b"}},
    (x,y) => Text.Lower(x) = Text.Lower(y))
