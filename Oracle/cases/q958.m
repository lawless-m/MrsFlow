// Table.AddColumn with nullable / list types.
// (Record-typed cells whose field-values are bracket-access expressions
// leave thunks in the cell record; mrsflow's Json.FromValue can't force
// them post-hoc. Excluded from this slice — record-cell forcing is a
// separate corner to address later.)
let t = Table.FromRecords({[a=1], [a=2]}) in
let r = try {
        Table.AddColumn(t, "nullable", each null, type nullable number),
        Table.AddColumn(t, "list", each {[a], [a]*2}, type list)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
