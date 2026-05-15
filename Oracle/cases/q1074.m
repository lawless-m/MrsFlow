// Duration.From with various sources.
let r = try {
        Duration.From("P1D"),
        Duration.From("P1DT2H30M"),
        Duration.From(1.5),
        Duration.From(#duration(1, 2, 3, 4)),
        Duration.From(null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
