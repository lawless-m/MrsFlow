// DateTime.From with number (OLE serial datetime).
let r = try {
        DateTime.From(45000),
        DateTime.From(45000.5),
        DateTime.From(0),
        DateTime.From(1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
