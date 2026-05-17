// Add 90 minutes → in next hour (the "next-hour" window
            // covers the hour AFTER this one). 65 mins from now is
            // squarely in next-hour even if it's "now+0..2 hours" sense.
            DateTime.IsInNextHour(
                DateTime.LocalNow() + #duration(0, 1, 5, 0))
