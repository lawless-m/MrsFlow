// is operator (M syntactic form for Type.Is).
let r = try {
        42 is number,
        "hello" is text,
        true is logical,
        null is null,
        42 is text,
        null is number,
        null is nullable number
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
