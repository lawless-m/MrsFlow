// Text.Format basic positional placeholders.
let r = try {
        Text.Format("hello #{0}", {"world"}),
        Text.Format("#{0} #{1}", {"hello", "world"}),
        Text.Format("#{1} #{0}", {"hello", "world"}),
        Text.Format("#{0}#{0}", {"X"}),
        Text.Format("no placeholders", {"X"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
