// InsertRows — insert one or more rows at offset.
            Table.InsertRows(
                Table.FromRecords({[a=1],[a=4]}),
                1, {[a=2],[a=3]})
