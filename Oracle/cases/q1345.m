// 5 months ago is in the previous 12 months → true.
            Date.IsInPreviousNMonths(
                Date.AddDays(DateTime.Date(DateTime.LocalNow()), -150), 12)
