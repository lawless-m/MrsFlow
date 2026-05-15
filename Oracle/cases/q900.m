// List.MaxN / MinN with N = 0, N > length.
let xs = {3, 1, 4, 1, 5} in
let r = try {
        List.MaxN(xs, 0),
        List.MaxN(xs, 100),
        List.MinN(xs, 0),
        List.MinN(xs, 100),
        List.MaxN({}, 3),
        List.MinN({}, 3)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
