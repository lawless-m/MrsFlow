// List.Skip variants.
let xs = {10, 20, 30, 40, 50} in
let r = try {
        List.Skip(xs, 0),
        List.Skip(xs, 2),
        List.Skip(xs, 5),
        List.Skip(xs, 10),
        List.Skip({}, 3),
        List.Skip(xs)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
