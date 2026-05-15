// List.RemoveItems with Comparer — built-in OrdinalIgnoreCase.
let r = try {
        List.RemoveItems({"a", "B", "c", "D"}, {"b", "d"}),
        List.RemoveItems({"a", "B", "c", "D"}, {"b", "d"}, Comparer.OrdinalIgnoreCase),
        List.RemoveItems({1, 2, 3, 4}, {2, 4}),
        List.RemoveItems({}, {1, 2})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
