// Whitespace handling — leading, trailing, both, tabs.
let r = try {
        Number.FromText("42"),
        Number.FromText(" 42"),
        Number.FromText("42 "),
        Number.FromText("  42  "),
        Number.FromText("#(tab)42#(tab)"),
        Number.FromText("3.14"),
        Number.FromText(" 3.14 "),
        try Number.FromText("") otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
