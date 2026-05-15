let r = try {Text.PadStart("42", 5), Text.PadStart("42", 5, "0"), Text.PadStart("hi", 2), Text.PadStart("", 3, "*")} in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
