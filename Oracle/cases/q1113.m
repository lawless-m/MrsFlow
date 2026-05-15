// Time + Duration wraparound: PQ wraps at 24:00 (no carry to date).
let r = try {
        #time(23, 0, 0) + #duration(0, 2, 0, 0),
        #time(1, 0, 0) - #duration(0, 2, 0, 0),
        #time(0, 0, 0) + #duration(0, 25, 0, 0),
        #time(12, 30, 0) + #duration(0, 0, 45, 0),
        #time(0, 0, 0) - #duration(0, 0, 0, 1)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
