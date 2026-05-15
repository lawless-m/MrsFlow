// Text.Split null inputs.
let r = try {
        Text.Split(null, ","),
        Text.Split("abc", null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
