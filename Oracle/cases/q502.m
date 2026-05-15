let r = try {
        List.MatchesAny({1, 2, 3}, each _ > 2),
        List.MatchesAny({1, 2, 3}, each _ > 10),
        List.MatchesAny({}, each _ > 0),
        List.MatchesAny({"a", "b", "c"}, each _ = "b")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
