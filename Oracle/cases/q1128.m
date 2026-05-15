// Deep nesting depth preserved across &.
let r = try {
        {{{1}}} & {{{2}}},
        {{{{1, 2}}}, 3} & {4},
        List.Count({{{1}}} & {{{2}}}),
        {{{1}}} & {{{2}}} & {{{3}}}
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
