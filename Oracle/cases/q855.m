// List.Sort with null elements — PQ sorts nulls first (less than everything)?
let r = try {
        List.Sort({3, null, 1, null, 2}),
        List.Sort({"b", null, "a"}),
        List.Sort({null, null}),
        List.Sort({null})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
