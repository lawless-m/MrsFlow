// Text.NewGuid format — derive observable properties without comparing
// random output (which would always diff).
let g = Text.NewGuid() in
let r = try {
        Text.Length(g) = 36,
        Text.PositionOf(g, "-") = 8,
        Text.PositionOf(g, "-", Occurrence.All) = {8, 13, 18, 23},
        Text.Length(Text.Replace(g, "-", "")) = 32,
        Text.Range(g, 14, 1) = "4"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
