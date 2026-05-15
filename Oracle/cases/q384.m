let r = try
        let c = Comparer.FromCulture("en-US", true) in
            {c("a", "A"), c("a", "B"), c("z", "a")}
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
