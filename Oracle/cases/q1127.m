// Equivalence: a & b == List.Combine({a, b}) for plain lists.
let r = try {
        ({1, 2} & {3, 4}) = List.Combine({{1, 2}, {3, 4}}),
        ({} & {1}) = List.Combine({{}, {1}}),
        ({{1}, 2} & {3}) = List.Combine({{{1}, 2}, {3}}),
        List.Count({1, 2, 3} & {4, 5}),
        List.Sum({1, 2} & {3, 4})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
