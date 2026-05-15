// List.Reverse on buffered list.
let r = try {
        List.Reverse(List.Buffer({1, 2, 3, 4, 5})),
        List.Reverse(List.Buffer({})),
        List.Reverse(List.Buffer({"a"})),
        List.Reverse(List.Buffer({null, 1, null}))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
