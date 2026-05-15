let r = try {
        List.Union({{1, 2, 3}, {3, 4, 5}}),
        List.Union({{1, 2}, {3, 4}, {5, 6}}),
        List.Union({{}, {1}}),
        List.Union({})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
