// Table.RemoveRowsWithErrors null inputs.
let r = try {
        Table.RemoveRowsWithErrors(null)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
