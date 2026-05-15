let r = try {
        Record.RemoveFields([a=1, b=2, c=3], "b"),
        Record.RemoveFields([a=1, b=2, c=3], {"a", "c"}),
        Record.RemoveFields([a=1, b=2, c=3], {})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
