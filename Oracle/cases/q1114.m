// Duration arithmetic: Duration + Duration, Duration - Duration, scalar * Duration.
let r = try {
        #duration(1, 0, 0, 0) + #duration(0, 12, 0, 0),
        #duration(2, 0, 0, 0) - #duration(0, 6, 0, 0),
        #duration(0, 1, 30, 0) + #duration(0, 0, 45, 30),
        3 * #duration(1, 0, 0, 0),
        #duration(7, 0, 0, 0) / 2
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
