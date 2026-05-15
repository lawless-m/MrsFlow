// #shared survives ToTable round-trip.
let t = Record.ToTable(#shared) in
let r = try {
        Table.ColumnNames(t),
        Table.RowCount(t) = Record.FieldCount(#shared),
        Table.RowCount(t) > 800
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
