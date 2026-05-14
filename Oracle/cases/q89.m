// Per the q128-family Oracle probes, PQ's Table.Group comparisonCriteria
// contract requires:
//   - bare key value passed to the callback (not a record)
//   - callback MUST return an ordering -1|0|1, not a logical
// Anything returning logical errors with "cannot convert true to Number".
Table.Group(
    #table({"k","v"}, {{"A",1},{"a",2},{"B",3}}),
    "k",
    {{"total", each List.Sum([v])}},
    GroupKind.Global,
    (a,b) => Value.Compare(Text.Lower(a), Text.Lower(b)))
