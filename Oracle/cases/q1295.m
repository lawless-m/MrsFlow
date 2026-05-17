// MatchesAllRows — predicate over rows.
            Table.MatchesAllRows(
                Table.FromRecords({[a=2],[a=4],[a=6]}),
                each Number.Mod(_[a], 2) = 0)
