// 10 seconds ago → in previous 60 seconds.
            DateTime.IsInPreviousNSeconds(
                DateTime.LocalNow() - #duration(0, 0, 0, 10), 60)
