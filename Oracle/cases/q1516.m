let r = try Excel.Workbook(#binary({1,2,3,4})) in
                if r[HasError] then [HasError=true, Reason=r[Error][Reason]] else [HasError=false, IsTable=Value.Is(r[Value], type table)]
