let r = try
        let
            rec = Record.AddField([], "x", () => error "computed!", true),
            v = Record.Field(rec, "x"),
            forced = try (if Value.Is(v, type function) then v() else v)
        in
            if forced[HasError] then "errored: " & forced[Error][Message] else "no error"
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
