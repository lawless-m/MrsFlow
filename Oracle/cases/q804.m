// Text.Trim where the trim char appears in the middle but is NOT touched
// (Trim only strips from boundaries).
let r = try {
        Text.Trim("XaXbX", "X"),
        Text.Trim("aXbXc", "X"),
        Text.TrimStart("aXbX", "X"),
        Text.TrimEnd("XaXb", "X")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
