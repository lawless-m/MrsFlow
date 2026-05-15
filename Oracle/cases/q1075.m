// #duration normalisation — hours > 24, minutes > 60.
let r = try {
        #duration(0, 25, 0, 0),
        #duration(0, 0, 90, 0),
        #duration(0, 0, 0, 3600),
        #duration(1, 24, 0, 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
