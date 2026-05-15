// List.Sort default — number ascending, text ascending.
let r = try {
        List.Sort({3, 1, 2}),
        List.Sort({"banana", "apple", "cherry"}),
        List.Sort({}),
        List.Sort({1}),
        List.Sort({3, 3, 1, 2, 1})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
