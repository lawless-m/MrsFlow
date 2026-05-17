// TransformRows — project each row through a function.
            // Returns a list (not a table) — Excel and mrsflow agree.
            Table.TransformRows(
                Table.FromRecords({[a=1],[a=2],[a=3]}),
                each [a] * 10)
