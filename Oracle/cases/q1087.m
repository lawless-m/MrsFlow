// Json.FromValue with NaN/Inf (per convention → null).
let r = try {
        Json.FromValue(0/0),
        Json.FromValue(1/0),
        Json.FromValue(-1/0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
