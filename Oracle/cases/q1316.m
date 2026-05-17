// ReplaceRows — replace a contiguous range of rows.
            Table.ReplaceRows(
                Table.FromRecords({[a=1],[a=2],[a=3],[a=4],[a=5]}),
                1, 2, {[a=99]})
