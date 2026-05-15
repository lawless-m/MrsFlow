// List.Combine deep / null elements.
let r = try {
        List.Combine({{{1, 2}}, {{3, 4}}}),
        List.Combine({{null, 1}, {2, null}}),
        List.Combine({{1, 2}, null})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
