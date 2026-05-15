let r = try
        let
            t1 = #table({"a", "b"}, {{"x", "y"}, {1, 2}}),
            demoted = Table.DemoteHeaders(t1),
            roundtrip = Table.PromoteHeaders(demoted)
        in
            roundtrip
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
