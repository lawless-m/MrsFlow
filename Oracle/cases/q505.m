let r = try
        let
            nullList = {null, null, null},
            mixedList = {1, null, 3}
        in
            {
                List.MatchesAll(nullList, each _ = null),
                List.MatchesAny(mixedList, each _ = null),
                List.MatchesAll(mixedList, each _ = null)
            }
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
