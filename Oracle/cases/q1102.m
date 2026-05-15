// null op X arithmetic — PQ rule: null + anything = null.
let r = try {
        null + 1,
        1 + null,
        null * 2,
        null - 3,
        null / 4,
        null + null
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
