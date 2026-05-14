List.Distinct({"a","A","b","B","c"},
    (x,y) => Text.Lower(x) = Text.Lower(y))
