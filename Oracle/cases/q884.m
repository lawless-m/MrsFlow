// List.FirstN / LastN predicate form: take while.
let xs = {1, 3, 5, 2, 4, 6} in
let r = try {
        List.FirstN(xs, each _ < 5),
        List.FirstN(xs, each _ > 0),
        List.FirstN(xs, each _ > 100),
        List.LastN(xs, each _ > 3),
        List.LastN(xs, each _ > 100)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
