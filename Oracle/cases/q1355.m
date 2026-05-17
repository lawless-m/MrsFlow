// 5 days before start of this month → previous month.
            Date.IsInPreviousMonth(
                Date.AddDays(Date.StartOfMonth(DateTime.Date(DateTime.LocalNow())), -5))
