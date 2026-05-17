// AddJoinColumn left-joins another table and stores its
            // rows in a new (nested) column.
            Table.AddJoinColumn(
                Table.FromRecords({[id=1],[id=2]}),
                "id",
                Table.FromRecords({[id=1, v="a"], [id=1, v="b"], [id=2, v="c"]}),
                "id",
                "joined")
