// List.Numbers default step (should be 1).
let r = try {
        List.Numbers(0, 5) = {0, 1, 2, 3, 4},
        List.Sum(List.Numbers(1, 100)) = 5050,
        List.Numbers(1.5, 3) = {1.5, 2.5, 3.5}
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
