// List.MaxN / MinN with duplicates / nulls.
let r = try {
        List.MaxN({1, 5, 5, 3, 5}, 3),
        List.MinN({1, 5, 5, 3, 5}, 3),
        List.MaxN({null, 1, null, 2, 3}, 2),
        List.MinN({null, 1, null, 2, 3}, 2),
        List.MaxN({null, null}, 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
