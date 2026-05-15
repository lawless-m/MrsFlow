// Text.Replace with a user lambda as 4th comparer arg — PQ may
// accept (like List.Sort with key) or reject (like List.Distinct).
let r = try {
        Text.Replace("Hello hello", "hello", "X", (a, b) => Value.Compare(Text.Lower(a), Text.Lower(b)))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
