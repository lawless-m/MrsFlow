// Lines.ToText custom separator.
let lines = {"a", "b", "c"} in
let r = try {
        Lines.ToText(lines),
        Lines.ToText(lines, "|"),
        Lines.ToText(lines, " - "),
        Lines.ToText({}, "|"),
        Lines.ToText({"single"}, "|")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
