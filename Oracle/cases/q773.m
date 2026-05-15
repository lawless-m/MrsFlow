// Text.SplitAny with unicode delimiters incl. surrogate-pair-territory chars
// (emoji are treated as single 'chars' in PQ's text-as-grapheme model).
let r = try {
        Text.SplitAny("a→b←c", "→←"),
        Text.SplitAny("café", "é"),
        Text.SplitAny("naïve", "ï"),
        Text.SplitAny("abc", null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
