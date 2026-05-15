// Record.RenameFields with conflicts.
let r = try {
        Record.RenameFields([a=1, b=2], {{"a", "x"}}),
        Record.RenameFields([a=1, b=2], {{"a", "x"}, {"b", "y"}}),
        // Renaming to an existing field name — should error.
        Record.RenameFields([a=1, b=2], {{"a", "b"}})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
