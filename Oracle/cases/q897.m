// Mixed-type list — does PQ refuse text-in-number list?
let r = try {
        List.Sum({1, 2, "3"}),
        List.Sum({1, true, 2})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
