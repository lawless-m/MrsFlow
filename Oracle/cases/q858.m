// List.Distinct with user lambda comparer — should be refused per
// Phase 1 boundary decision (PQ rejects user lambdas in Distinct).
let r = try {
        List.Distinct({"a", "A"}, (x, y) => Text.Lower(x) = Text.Lower(y))
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
