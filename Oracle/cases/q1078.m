// Duration arithmetic and totals. Pick a duration whose Total* values
// are all f64-exact (sums of powers of 2) so the comparison doesn't
// hit a 1-ulp libm divergence.
let d = #duration(1, 12, 0, 0) in
let r = try {
        Duration.TotalDays(d),
        Duration.TotalHours(d),
        Duration.TotalMinutes(d),
        Duration.Days(d),
        Duration.Hours(d),
        Duration.Minutes(d)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
