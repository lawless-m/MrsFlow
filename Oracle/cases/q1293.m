// IsDistinct — true if all rows are unique.
            { Table.IsDistinct(
                Table.FromRecords({[a=1],[a=2],[a=3]})),
              Table.IsDistinct(
                Table.FromRecords({[a=1],[a=2],[a=1]})) }
