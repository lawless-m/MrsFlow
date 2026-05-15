// List.Buffer on lazy/generated lists.
let r = try {
        List.Buffer(List.Generate(() => 0, (s) => s < 5, (s) => s + 1)),
        List.Buffer(List.Numbers(0, 5)),
        List.Sum(List.Buffer(List.Numbers(1, 10))) = 55
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
