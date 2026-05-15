// JSON serialisation: integer vs float-with-trailing-zero around boundary.
let r = try {
        Json.FromValue(999999999999999),
        Json.FromValue(1000000000000000),
        Json.FromValue(9007199254740992),
        Json.FromValue(1.5e15)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
