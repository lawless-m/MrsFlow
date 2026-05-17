// HasColumns — true iff all named columns exist.
            { Table.HasColumns(
                Table.FromRecords({[a=1,b=2]}), {"a","b"}),
              Table.HasColumns(
                Table.FromRecords({[a=1,b=2]}), {"a","c"}) }
