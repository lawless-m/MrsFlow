// Text.Lower/Upper lt-LT — Lithuanian preserves dot in some lowercase forms,
// but at .NET level the default ToLower("I") = "i" anyway. Verify.
let r = try {
        Text.Upper("istanbul", "lt-LT"),
        Text.Lower("ISTANBUL", "lt-LT"),
        Text.Upper("ąčęėįšų", "lt-LT")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
