// RemoveRows drops a range of rows by offset+count.
            Table.RemoveRows(
                Table.FromRecords({[a=1],[a=2],[a=3],[a=4],[a=5]}),
                1, 2)
