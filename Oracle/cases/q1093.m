// Lines.FromText with mixed separators.
let r = try {
        Lines.FromText("a#(cr,lf)b#(lf)c#(cr)d"),
        Lines.FromText("a#(lf)b"),
        Lines.FromText("a#(cr)b"),
        Lines.FromText(""),
        Lines.FromText("single")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
