// Table.ReplaceValue on a column that has error cells: targets values,
// not error markers (unless 'value' arg coincides with cell content).
let t = Table.FromRecords({[a=1], [a=2], [a=3]}) in
let t2 = Table.AddColumn(t, "b", each if [a] = 2 then error "oops" else [a] * 10) in
let r = try {
        Table.ReplaceValue(
            Table.ReplaceErrorValues(t2, {{"b", -1}}),
            -1, 99, Replacer.ReplaceValue, {"b"}
        ),
        Table.ReplaceValue(
            Table.ReplaceErrorValues(t2, {{"b", -1}}),
            10, 100, Replacer.ReplaceValue, {"b"}
        )
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
