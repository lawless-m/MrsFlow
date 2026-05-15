// List.Distinct with record / list elements — structural equality.
let r = try {
        List.Distinct({[a=1], [a=2], [a=1]}),
        List.Distinct({{1, 2}, {1, 2}, {1, 3}}),
        List.Distinct({[a=1, b=2], [b=2, a=1]})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
