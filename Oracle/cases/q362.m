let r = try {Text.PadEnd("42", 5), Text.PadEnd("42", 5, "."), Text.PadEnd("hi", 2), Text.PadEnd("", 3, "x")} in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
