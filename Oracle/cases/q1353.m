// Add 1 year via 366 days then trim back so it lands
            // in next calendar year.
            Date.IsInNextYear(
                Date.AddDays(Date.EndOfYear(DateTime.Date(DateTime.LocalNow())), 5))
