// Partition splits the table into N groups by a key column,
            // each group as a sub-table. Returns a list of tables.
            Table.Partition(
                Table.FromRecords({[g=1,x=1],[g=2,x=2],[g=1,x=3],[g=2,x=4]}),
                "g", 2, each _)
