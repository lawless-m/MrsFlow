let r = try {
        List.AnyTrue({true, false, false}),
        List.AnyTrue({false, false, false}),
        List.AnyTrue({true, true, true}),
        List.AnyTrue({})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
