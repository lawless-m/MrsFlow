// List.Durations generator.
let r = try {
        List.Durations(#duration(0, 0, 0, 0), 5, #duration(0, 1, 0, 0)),
        List.Durations(#duration(0, 0, 0, 0), 3, #duration(0, 0, 15, 0)),
        List.Durations(#duration(1, 0, 0, 0), 4, #duration(0, 12, 0, 0))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
