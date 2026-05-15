let r = try
        let
            buffered = List.Buffer({"x", "y", "z"}),
            first = buffered{0},
            last = buffered{2}
        in
            first & "-" & last
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
