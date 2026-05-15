// Table.FromRecords basic + heterogeneous-field records.
let r = try {
        Table.FromRecords({[a=1, b=2], [a=3, b=4]}),
        Table.FromRecords({}),
        Table.FromRecords({[a=1]}),
        // Heterogeneous-schema records — does PQ union or error?
        Table.FromRecords({[a=1, b=2], [a=3, c=4]})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
