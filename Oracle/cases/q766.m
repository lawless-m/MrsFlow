// Text.Replace with null inputs — null text / old / new arg.
let r = try {
        Text.Replace(null, "a", "b"),
        Text.Replace("abc", null, "b"),
        Text.Replace("abc", "a", null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
