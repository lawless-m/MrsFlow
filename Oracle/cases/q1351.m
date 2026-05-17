// 35 days ahead is in next month. To be safely in
            // *next* month not *month after next*, use a small
            // offset within the next month — 40 days catches at
            // least part of a 30-day next month.
            Date.IsInNextMonth(
                Date.AddDays(Date.EndOfMonth(DateTime.Date(DateTime.LocalNow())), 5))
