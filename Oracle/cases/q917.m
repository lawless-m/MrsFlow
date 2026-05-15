// List.Buffer compose with Reverse / FirstN.
let xs = List.Buffer({5, 2, 8, 1, 9}) in
let r = try {
        List.Reverse(xs),
        List.FirstN(xs, 3),
        List.LastN(xs, 2),
        List.Sort(xs)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
