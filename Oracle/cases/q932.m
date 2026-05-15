// List.Select with index — does PQ support indexed predicate?
let r = try {
        List.Select({10, 20, 30, 40}, each _ > 15),
        List.Select({}, each _ > 0),
        List.Select({10, 20, 30, 40}, (v, i) => i > 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
