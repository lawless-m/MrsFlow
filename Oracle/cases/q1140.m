// Table.FillDown over an error-then-replaced column.
let t = Table.FromRecords({[a=1], [a=null], [a=null], [a=4], [a=null]}) in
let t2 = Table.AddColumn(t, "b", each if [a] = 4 then error "oops" else [a]) in
let r = try {
        Table.FillDown(t, {"a"}),
        Table.FillDown(Table.ReplaceErrorValues(t2, {{"b", -1}}), {"b"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
