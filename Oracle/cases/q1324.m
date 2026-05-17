// AggregateTableColumn — given a column of nested tables,
            // collapse each by aggregations across one of its columns.
            Table.AggregateTableColumn(
                Table.FromRecords({
                    [g=1, nested=Table.FromRecords({[v=10],[v=20]})],
                    [g=2, nested=Table.FromRecords({[v=30]})] }),
                "nested",
                {{"v", List.Sum, "total"}})
