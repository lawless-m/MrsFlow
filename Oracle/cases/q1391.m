// DateTimeZone.UtcNow's offset is always 0. Probe via
            // ZoneHours of UtcNow().
            DateTimeZone.ZoneHours(DateTimeZone.UtcNow())
