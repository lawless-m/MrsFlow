let r = try
        let
            orig = [x=10, y=20, z=30],
            asList = Record.ToList(orig),
            roundtrip = Record.FromList(asList, Record.FieldNames(orig))
        in
            Record.FieldValues(roundtrip)
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
