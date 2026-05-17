// ~100 days ahead is in next quarter.
            Date.IsInNextQuarter(
                Date.AddDays(Date.EndOfQuarter(DateTime.Date(DateTime.LocalNow())), 5))
