let r = try
        let
            original = [a=1, b=2, c=3, d=4],
            step1 = Record.RemoveFields(original, "d"),
            step2 = Record.RenameFields(step1, {{"a", "alpha"}, {"c", "charlie"}}),
            step3 = Record.TransformFields(step2, {{"alpha", each _ * 100}})
        in
            step3
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
