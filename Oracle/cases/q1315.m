// ReplaceMatchingRows — swap matching rows with replacement.
            Table.ReplaceMatchingRows(
                Table.FromRecords({[a=1],[a=2],[a=3]}),
                {{[a=2], [a=99]}})
