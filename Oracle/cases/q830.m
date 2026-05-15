// Doubled ## — does PQ treat this as escape for literal #?
let r = try {
        Text.Format("##", {"X"}),
        Text.Format("##{0}", {"X"}),
        Text.Format("###{0}", {"X"}),
        Text.Format("####", {"X"}),
        Text.Format("a##b", {"X"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
