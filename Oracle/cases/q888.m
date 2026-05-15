// List.Zip null inputs and non-list elements.
let r = try {
        List.Zip(null),
        List.Zip({null}),
        List.Zip({{1, 2}, null}),
        List.Zip({"not-a-list"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
