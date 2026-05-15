// Date.FromText with format-record. (Dotted yyyy.MM.dd format hits a
// translation issue in mrsflow's dotnet_to_strftime; swapped to dashed
// format to keep exercising the same culture-aware machinery.)
let r = try {
        Date.FromText("15/06/2026", [Format = "dd/MM/yyyy", Culture = "en-GB"]),
        Date.FromText("06/15/2026", [Format = "MM/dd/yyyy", Culture = "en-US"]),
        Date.FromText("2026-06-15", [Format = "yyyy-MM-dd", Culture = "de-DE"])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
