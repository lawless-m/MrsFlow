// 5 days before start of this quarter → previous quarter.
            Date.IsInPreviousQuarter(
                Date.AddDays(Date.StartOfQuarter(DateTime.Date(DateTime.LocalNow())), -5))
