// PrefixColumns prefixes every column name with `<prefix>.`.
            Table.PrefixColumns(
                Table.FromRecords({[a=1,b=2]}),
                "x")
