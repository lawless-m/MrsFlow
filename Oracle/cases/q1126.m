// List & with records and tables as elements (still concatenation, no deep merge).
let r = try {
        {[a=1]} & {[b=2]},
        {[a=1], [a=2]} & {[a=3]},
        {{1, 2}} & {{1, 2}},
        {1} & {1.0},
        {true, false} & {true}
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
