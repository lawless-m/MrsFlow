// #shared has no duplicate field names (would error on Record access otherwise).
let names = Record.FieldNames(#shared) in
let r = try {
        List.Count(names) = List.Count(List.Distinct(names)),
        List.Count(names),
        List.Count(List.Distinct(names))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
