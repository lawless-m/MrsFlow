// ToList converts each row to a single value via combiner.
            // All-text row content so the default combiner works.
            Table.ToList(
                Table.FromRecords({[a="1",b="x"],[a="2",b="y"]}),
                Combiner.CombineTextByDelimiter(":"))
