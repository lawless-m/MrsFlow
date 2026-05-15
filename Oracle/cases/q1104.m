// null op X logical.
let r = try {
        null and true,
        null and false,
        true and null,
        false and null,
        null or true,
        null or false,
        not null
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
