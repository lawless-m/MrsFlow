// Overlapping needles — PQ scans left-to-right without re-scanning the
// match position; "aaaa" / "aa" gives matches at 0 and 2, not 0,1,2.
let r = try {
        Text.PositionOf("aaaa", "aa", Occurrence.All),
        Text.PositionOf("aaaaa", "aa", Occurrence.All),
        Text.PositionOf("aaa", "aa", Occurrence.All),
        Text.PositionOf("ababab", "aba", Occurrence.All)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
