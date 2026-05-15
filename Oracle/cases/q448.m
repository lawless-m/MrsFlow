let r = try Date.FromText("06/15/2024", [Format="MM/dd/yyyy", Culture="en-US"]) in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
