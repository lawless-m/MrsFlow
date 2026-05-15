let r = try {
        List.MatchesAll({1, 2, 3}, each _ > 0),
        List.MatchesAll({1, -2, 3}, each _ > 0),
        List.MatchesAll({}, each _ > 0),
        List.MatchesAll({1, 1, 1}, each _ = 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
