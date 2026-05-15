// List.Sort with key selector — sort by computed key.
let r = try {
        List.Sort({"banana", "apple", "cherry", "date"}, each Text.Length(_)),
        List.Sort({"apple", "banana", "cherry"}, each Text.Length(_)),
        List.Sort({3.5, 1.2, 2.8, 4.0}, each Number.Round(_)),
        List.Sort({-3, -1, 2, -5, 4}, each Number.Abs(_))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
