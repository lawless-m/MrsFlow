// List.Mode / List.Modes.
let r = try {
        List.Mode({}),
        List.Mode({null}),
        List.Mode({1, 2, 2, 3, 3, 3}),
        List.Mode({1, 2, 3}),
        List.Modes({1, 2, 2, 3, 3}),
        List.Modes({1, 2, 3})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
