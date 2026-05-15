// List.Distinct with null arg + non-list arg.
let r = try {
        List.Distinct(null),
        List.Distinct("abc")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
