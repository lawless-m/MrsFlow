let r = try {
        List.Intersect({{1, 2, 3, 4}, {2, 3, 5}}),
        List.Intersect({{1, 2, 3}, {4, 5, 6}}),
        List.Intersect({{1, 2}, {1, 2}, {1, 2}}),
        List.Intersect({{1, 2, 3}, {}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
