let r = try
        let
            g = Text.NewGuid(),
            parts = Text.Split(g, "-")
        in
            List.Transform(parts, each Text.Length(_))
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
