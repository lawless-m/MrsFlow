// SelectRowsWithErrors filters to rows containing any error
            // cells. A plain Table.FromRecords has none.
            Table.SelectRowsWithErrors(
                Table.FromRecords({[a=1],[a=2],[a=3]}))
