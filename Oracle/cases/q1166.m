let
                Source = #table(
                    type table [d = datetime],
                    {
                        { #datetime(2025, 6, 15, 10, 0, 0) },
                        { #datetime(2026, 6, 15, 10, 0, 0) }
                    }
                ),
                Filtered = Table.SelectRows(Source, each [d] > #date(2026, 1, 1))
            in
                Table.RowCount(Filtered)
