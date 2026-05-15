// List.Generate with selector (4th arg) — projection from state.
let r = try {
        List.Generate(() => 0, (s) => s < 5, (s) => s + 1, (s) => s * s),
        List.Generate(() => 1, (s) => s <= 10, (s) => s + 1, (s) => Text.From(s)),
        List.Generate(() => 0, (s) => s < 0, (s) => s + 1, (s) => s * 100)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
