let r = try {
        try Record.RemoveFields([a=1, b=2], "z") otherwise "err",
        try Record.RenameFields([a=1], {{"x", "y"}}) otherwise "err",
        try Record.TransformFields([a=1], {{"z", each _ * 2}}) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
