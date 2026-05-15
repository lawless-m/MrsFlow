// List.MaxN / List.MinN basic — top/bottom N.
let xs = {3, 1, 4, 1, 5, 9, 2, 6} in
let r = try {
        List.MaxN(xs, 1),
        List.MaxN(xs, 3),
        List.MaxN(xs, 5),
        List.MinN(xs, 1),
        List.MinN(xs, 3),
        List.MinN(xs, 5)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
