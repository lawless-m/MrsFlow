// List.FirstN / LastN on empty list, with negative count, with predicate.
let r = try {
        List.FirstN({}, 3),
        List.LastN({}, 3),
        List.FirstN({1, 2, 3}, -1),
        List.LastN({1, 2, 3}, -1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
