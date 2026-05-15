let r = try List.Sum(List.Buffer({1, 2, 3, 4, 5})) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
