// List.FirstN / List.LastN with various counts.
let xs = {10, 20, 30, 40, 50} in
let r = try {
        List.FirstN(xs, 0),
        List.FirstN(xs, 2),
        List.FirstN(xs, 5),
        List.FirstN(xs, 10),
        List.LastN(xs, 0),
        List.LastN(xs, 2),
        List.LastN(xs, 5),
        List.LastN(xs, 10)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
