Table.RowCount(Table.AddKey(
                Table.FromRecords({[a=1, b="x"],[a=2, b="y"]}),
                {"a"}, true))
