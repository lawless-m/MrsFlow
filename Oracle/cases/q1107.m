// Text + Text / Text + number / List + List / Record + Record.
let r = try {
        "hello" & " world",
        "a" & "b" & "c",
        {1, 2} & {3, 4},
        [a=1] & [b=2],
        [a=1] & [a=2]
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
