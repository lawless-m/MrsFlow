let r = try {
        List.AllTrue({true, true, true}),
        List.AllTrue({true, false, true}),
        List.AllTrue({false, false}),
        List.AllTrue({})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
