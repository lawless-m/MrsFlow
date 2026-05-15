let r = try
        let
            lst = {1, 2, 3, 4, 5},
            allEven = List.MatchesAll(lst, each Number.Mod(_, 2) = 0),
            anyEven = List.MatchesAny(lst, each Number.Mod(_, 2) = 0)
        in
            {allEven, anyEven}
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
