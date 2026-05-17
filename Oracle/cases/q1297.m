// AlternateRows — drop every other row starting at offset.
            Table.AlternateRows(
                Table.FromRecords({[a=1],[a=2],[a=3],[a=4],[a=5],[a=6]}),
                0, 1, 1)
