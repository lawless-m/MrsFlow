// ContainsAll — true if EVERY supplied row exists.
            Table.ContainsAll(
                Table.FromRecords({[a=1,b=2],[a=3,b=4],[a=5,b=6]}),
                {[a=1,b=2], [a=5,b=6]})
