// 5 days from now is in the next 30 days → true.
            Date.IsInNextNDays(
                Date.AddDays(DateTime.Date(DateTime.LocalNow()), 5), 30)
