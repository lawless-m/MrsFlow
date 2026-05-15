let r = try Date.FromText("15/06/2024", [Format="dd/MM/yyyy", Culture="en-GB"]) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
