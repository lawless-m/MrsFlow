// DuplicateColumn copies a column under a new name.
            Table.DuplicateColumn(
                Table.FromRecords({[a=1,b=2],[a=3,b=4]}),
                "a", "a_copy")
