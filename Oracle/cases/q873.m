// List.Generate with condition false at seed → empty list.
let r = try {
        List.Generate(() => 100, (s) => s < 5, (s) => s + 1),
        List.Generate(() => null, (s) => s <> null, (s) => s)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
