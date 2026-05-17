// FindText returns rows where any column's text-form contains
            // the substring.
            Table.FindText(
                Table.FromRecords({[name="apple"],[name="banana"],[name="apricot"]}),
                "ap")
