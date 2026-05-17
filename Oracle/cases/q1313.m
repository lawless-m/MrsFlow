// Split — chunk into N-row sub-tables.
            Table.Split(
                Table.FromRecords({[a=1],[a=2],[a=3],[a=4],[a=5]}),
                2)
