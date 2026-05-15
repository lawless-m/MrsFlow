// Family-size order-of-magnitude check. Exact PQ counts are
// version-sensitive (which "Edge Browser" or "AzureML" name gets
// added/removed each PQ release shifts a single family up/down by 1),
// so we sanity-check the shape rather than byte-exact totals.
let names = Record.FieldNames(#shared) in
let countPrefix = (p) => List.Count(List.Select(names, each Text.StartsWith(_, p))) in
let r = try {
        countPrefix("Text.") > 30,
        countPrefix("List.") > 60,
        countPrefix("Table.") > 100,
        countPrefix("Record.") > 15,
        countPrefix("Number.") > 40,
        countPrefix("Date.") > 50
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
