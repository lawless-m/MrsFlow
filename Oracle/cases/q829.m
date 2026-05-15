// Text.Format with literal # — what does PQ do with a bare # not followed by {?
let r = try {
        Text.Format("price: #25", {"X"}),
        Text.Format("a#b", {"X"}),
        Text.Format("#", {"X"}),
        Text.Format("# is literal", {"X"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
