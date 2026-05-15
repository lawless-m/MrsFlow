// 10 calls all distinct.
let guids = List.Generate(() => 0, (i) => i < 10, (i) => i + 1, (i) => Text.NewGuid()) in
let r = try {
        List.Count(List.Distinct(guids)) = 10
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
