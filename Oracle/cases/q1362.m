// 1 quarter ahead → in next 4 quarters.
            Date.IsInNextNQuarters(
                Date.AddDays(Date.EndOfQuarter(DateTime.Date(DateTime.LocalNow())), 5), 4)
