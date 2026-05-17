// MatchesAnyRows.
            Table.MatchesAnyRows(
                Table.FromRecords({[a=1],[a=3],[a=4]}),
                each _[a] > 3)
