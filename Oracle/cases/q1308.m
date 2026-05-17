// ToColumns transposes a table into a list of columns
            // (each column being a list of cell values).
            Table.ToColumns(
                Table.FromRecords({[a=1,b=10],[a=2,b=20],[a=3,b=30]}))
