let r = try {
        List.Contains({"a", "b", "c"}, "b"),
        List.Contains({"a", "b", "c"}, "z"),
        List.Contains({}, "x"),
        List.Contains({1, 2, null}, null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
