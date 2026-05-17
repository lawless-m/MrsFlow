// 2 quarters ago → in previous 4 quarters.
            Date.IsInPreviousNQuarters(
                Date.AddDays(Date.StartOfQuarter(
                    DateTime.Date(DateTime.LocalNow())), -180), 4)
