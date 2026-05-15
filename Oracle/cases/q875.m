// List.Generate identity properties.
let r = try {
        List.Count(List.Generate(() => 0, (s) => s < 10, (s) => s + 1)) = 10,
        List.Sum(List.Generate(() => 1, (s) => s <= 5, (s) => s + 1)) = 15
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
