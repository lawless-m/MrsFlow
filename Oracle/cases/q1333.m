// 5 days ago is in the previous 30 days → true.
            Date.IsInPreviousNDays(
                Date.AddDays(DateTime.Date(DateTime.LocalNow()), -5), 30)
