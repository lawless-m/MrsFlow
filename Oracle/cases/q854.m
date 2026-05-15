// List.Sort with comparer lambda returning -1/0/1.
let r = try {
        List.Sort({3, 1, 2}, (a, b) => Value.Compare(a, b)),
        List.Sort({3, 1, 2}, (a, b) => -Value.Compare(a, b)),
        List.Sort({"banana", "apple"}, (a, b) => Value.Compare(Text.Lower(a), Text.Lower(b)))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
