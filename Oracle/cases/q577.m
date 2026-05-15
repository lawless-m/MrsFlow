let r = try {
        Record.RenameFields([a=1, b=2, c=3], {{"a", "alpha"}}),
        Record.RenameFields([a=1, b=2, c=3], {{"a", "x"}, {"c", "z"}}),
        Record.RenameFields([a=1, b=2], {})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
