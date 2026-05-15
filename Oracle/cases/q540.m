let r = try Text.FromBinary(Json.FromValue([
        quote = "he said ""hi""",
        backslash = "C:\path\file",
        tab = "a#(tab)b",
        newline = "line1#(lf)line2"
    ]), TextEncoding.Utf8) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
