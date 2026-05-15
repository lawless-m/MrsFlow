// Record.AddField with collision — adding a field that already exists.
let r = try {
        Record.AddField([a=1], "a", 99)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
