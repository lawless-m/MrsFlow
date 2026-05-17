// ToRecords yields the row-list back out — round-trips
            // through Table.FromRecords/Table.ToRecords.
            Table.ToRecords(
                Table.FromRecords({[a=1,b=2],[a=3,b=4]}))
