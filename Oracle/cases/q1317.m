// TransformColumnNames maps each column name with a fn.
            Table.TransformColumnNames(
                Table.FromRecords({[a=1, b=2]}),
                each Text.Upper(_))
