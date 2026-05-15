// D with negatives — minus sign before padded digits, .NET .net convention
// is "-00042" not "-42 padded".
let r = try {
        Number.ToText(-42, "D"),
        Number.ToText(-42, "D0"),
        Number.ToText(-42, "D5"),
        Number.ToText(-42, "D10"),
        Number.ToText(-7, "D3"),
        Number.ToText(-0, "D5"),
        Number.ToText(-1, "D5"),
        Number.ToText(-99999, "D5")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
