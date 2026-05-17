// ContainsAny — true if ANY supplied row exists.
            Table.ContainsAny(
                Table.FromRecords({[a=1,b=2],[a=3,b=4]}),
                {[a=99,b=99], [a=1,b=2]})
