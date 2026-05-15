// Text.Trim default — what whitespace does PQ consider trimmable?
// Test ASCII space, tab, CR, LF, NBSP, U+2028.
let r = try {
        Text.Trim("  abc  "),
        Text.Trim("#(tab)abc#(tab)"),
        Text.Trim("#(cr,lf)abc#(cr,lf)"),
        Text.Trim("#(00A0)abc#(00A0)"),
        Text.Trim("#(2028)abc#(2028)"),
        Text.Trim("abc")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
