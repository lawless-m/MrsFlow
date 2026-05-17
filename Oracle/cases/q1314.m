// RemoveMatchingRows — drop rows matching any of the given
            // record predicates.
            Table.RemoveMatchingRows(
                Table.FromRecords({[a=1,b=2],[a=3,b=4],[a=5,b=6]}),
                {[a=3,b=4]})
