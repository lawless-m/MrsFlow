let r = try
        let
            rec = Record.AddField([], "computed", () => 10 * 3, true),
            v = Record.Field(rec, "computed"),
            forced = if Value.Is(v, type function) then v() else v
        in
            forced
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
