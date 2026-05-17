// 5 weeks ago is in the previous 10 weeks → true.
            Date.IsInPreviousNWeeks(
                Date.AddDays(DateTime.Date(DateTime.LocalNow()), -35), 10)
