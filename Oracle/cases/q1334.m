// 100 days ago is NOT in the previous 30 days → false.
            Date.IsInPreviousNDays(
                Date.AddDays(DateTime.Date(DateTime.LocalNow()), -100), 30)
