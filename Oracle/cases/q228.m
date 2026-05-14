Table.AddColumn(#table({"A"}, {{1}}), "label", each "row-" & Text.From([A]), type text)
