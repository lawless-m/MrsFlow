let r = try
        let
            g = Text.NewGuid(),
            lower = Text.Lower(g),
            isHex = List.AllTrue(List.Transform(Text.ToList(Text.Replace(lower, "-", "")), each Text.Contains("0123456789abcdef", _)))
        in
            isHex
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
