// List.Combine flattens ONE level: {{1,2},{3,4}} → {1,2,3,4}.
// {{1,{2}}, {3}} → {1,{2},3} (inner stays nested).
let r = try {
        List.Combine({{1, 2}, {3, 4}}),
        List.Combine({{1, {2}}, {3}}),
        List.Combine({}),
        List.Combine({{}, {}}),
        List.Combine({{1}, {}, {2, 3}}),
        List.Combine({{null, 1}, {2, null}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
