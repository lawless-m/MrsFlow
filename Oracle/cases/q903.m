// List.Numbers — generator: start, count, [step].
let r = try {
        List.Numbers(1, 5),
        List.Numbers(0, 0),
        List.Numbers(5, 3),
        List.Numbers(1, 5, 2),
        List.Numbers(10, 4, -1),
        List.Numbers(0, 3, 0.5),
        List.Numbers(1, 5, -2)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
