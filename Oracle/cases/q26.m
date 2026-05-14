Table.TransformColumns(
    #table({"A"}, {{5}}),
    {{"A", each _ + 1, Int64.Type}})
