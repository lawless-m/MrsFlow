// Spot-check core function names exist in #shared.
let names = Record.FieldNames(#shared) in
let r = try {
        List.Contains(names, "Text.From"),
        List.Contains(names, "Number.From"),
        List.Contains(names, "List.Generate"),
        List.Contains(names, "Table.Group"),
        List.Contains(names, "Json.Document"),
        List.Contains(names, "Csv.Document"),
        List.Contains(names, "Date.AddDays")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
