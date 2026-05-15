// D with float inputs — .NET says D only valid for integral types,
// but PQ's Number.ToText accepts floats. Check what happens with non-integers.
let r = try {
        try Number.ToText(3.7, "D") otherwise "err",
        try Number.ToText(3.7, "D5") otherwise "err",
        try Number.ToText(-3.7, "D5") otherwise "err",
        try Number.ToText(0.5, "D5") otherwise "err",
        try Number.ToText(-0.5, "D5") otherwise "err",
        try Number.ToText(3.0, "D5") otherwise "err",
        try Number.ToText(0.0, "D5") otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
