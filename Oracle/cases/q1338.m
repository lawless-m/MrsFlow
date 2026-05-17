// 400 days ago is NOT in the current year.
            Date.IsInCurrentYear(
                Date.AddDays(DateTime.Date(DateTime.LocalNow()), -400))
