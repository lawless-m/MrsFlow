// Record.FromList with mismatched lengths.
let r = try {
        Record.FromList({1, 2, 3}, {"a", "b"}),
        Record.FromList({1, 2}, {"a", "b", "c"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
