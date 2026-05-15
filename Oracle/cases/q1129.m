// List.Combine refuses non-list elements (mixed types in outer list).
let r = try {
        List.Combine({1, 2, 3}),
        List.Combine({{1}, 2, {3}}),
        List.Combine({{1}, null, {3}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
