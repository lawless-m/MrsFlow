let r = try {
        List.RemoveNulls({1, null, 2, null, 3}),
        List.RemoveNulls({null, null}),
        List.RemoveNulls({1, 2, 3}),
        List.RemoveNulls({})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
