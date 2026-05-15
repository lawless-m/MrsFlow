// Lexical structure — segments of length 8-4-4-4-12.
let g = Text.NewGuid() in
let parts = Text.Split(g, "-") in
let r = try {
        List.Count(parts) = 5,
        Text.Length(parts{0}) = 8,
        Text.Length(parts{1}) = 4,
        Text.Length(parts{2}) = 4,
        Text.Length(parts{3}) = 4,
        Text.Length(parts{4}) = 12
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
