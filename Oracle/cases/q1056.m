// Record.FromList basic — equal-length lists.
let r = try {
        Record.FromList({1, 2, 3}, {"a", "b", "c"}),
        Record.FromList({}, {}),
        Record.FromList({"x"}, {"name"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
