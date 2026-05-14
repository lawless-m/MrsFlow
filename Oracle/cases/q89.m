// Table.Group comparisonCriteria receives (a,b) which are *equality
// callbacks* between key tuples — our impl uses rows_equal_with_criteria,
// so the function must return logical (not a Value.Compare ordering).
Table.Group(
    #table({"k","v"}, {{"A",1},{"a",2},{"B",3}}),
    "k",
    {{"total", each List.Sum([v])}},
    GroupKind.Global,
    (a,b) => Text.Lower(a[k]) = Text.Lower(b[k]))
