// List & List preserves nesting (no flattening of inner lists).
let r = try {
        {1, {2, 3}} & {{4}, 5},
        {} & {1, 2},
        {1, 2} & {},
        {} & {},
        {{1}} & {{2}},
        {null, 1} & {2, null}
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
