let r = try {
        List.IsEmpty({}),
        List.IsEmpty({1}),
        List.IsEmpty({null}),
        List.IsEmpty({""})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
