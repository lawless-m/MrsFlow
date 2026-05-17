// 5 days before start of this year → previous year.
            Date.IsInPreviousYear(
                Date.AddDays(Date.StartOfYear(DateTime.Date(DateTime.LocalNow())), -5))
