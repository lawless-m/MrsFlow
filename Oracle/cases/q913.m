// List.Buffer basic — round-trips contents.
let r = try {
        List.Buffer({1, 2, 3}),
        List.Buffer({}),
        List.Buffer({"a", "b"}),
        List.Buffer({null, 1, null}),
        List.Buffer({{1, 2}, {3, 4}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
