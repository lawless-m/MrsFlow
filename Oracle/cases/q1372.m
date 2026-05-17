// 5 minutes ago → in previous 30 minutes.
            DateTime.IsInPreviousNMinutes(
                DateTime.LocalNow() - #duration(0, 0, 5, 0), 30)
