// List.Sort with Order.Descending.
let r = try {
        List.Sort({3, 1, 2}, Order.Descending),
        List.Sort({"banana", "apple", "cherry"}, Order.Descending),
        List.Sort({1, 2, 3}, Order.Ascending),
        List.Sort({3, 2, 1}, Order.Ascending),
        List.Sort({1}, Order.Descending),
        List.Sort({}, Order.Descending)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
