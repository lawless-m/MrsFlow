// Distribution sanity — 1000 RandomBetween(0,100) values should land
// in range and average somewhere near 50 (loose: 30..70).
let xs = List.Transform(List.Numbers(1, 1000), each Number.RandomBetween(0, 100)) in
let r = try {
        List.Max(xs) <= 100,
        List.Min(xs) >= 0,
        List.Average(xs) > 30 and List.Average(xs) < 70,
        List.Count(List.Distinct(xs)) > 500
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
