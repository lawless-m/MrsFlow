// 100 days from now is NOT in the next 30 → false.
            Date.IsInNextNDays(
                Date.AddDays(DateTime.Date(DateTime.LocalNow()), 100), 30)
