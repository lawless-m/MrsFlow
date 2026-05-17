// Transpose swaps rows and columns. Column names of the
            // result are auto-generated.
            Table.Transpose(
                Table.FromRecords({[a=1,b=2,c=3],[a=4,b=5,c=6]}))
