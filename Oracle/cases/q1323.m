// StopFolding is a passthrough that prevents downstream
            // query-folding into the source. Result equals input.
            Table.StopFolding(
                Table.FromRecords({[a=1],[a=2]}))
