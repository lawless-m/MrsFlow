// List.Combine with non-list elements — does PQ refuse or coerce?
let r = try {
        List.Combine({{1, 2}, {3, 4}}),
        List.Combine({}),
        List.Combine({{1, 2}}),
        List.Combine({{}, {}, {}}),
        List.Combine({{1, 2}, "abc"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
