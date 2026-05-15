// List.Range — offset, optional count.
let xs = {10, 20, 30, 40, 50} in
let r = try {
        List.Range(xs, 0),
        List.Range(xs, 2),
        List.Range(xs, 5),
        List.Range(xs, 0, 2),
        List.Range(xs, 1, 3),
        List.Range(xs, 3, 10),
        List.Range(xs, 0, 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
