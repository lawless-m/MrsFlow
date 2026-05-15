// Duration arithmetic: addition, subtraction, comparison.
let r = try {
        #duration(1, 0, 0, 0) + #duration(0, 12, 0, 0),
        #duration(2, 0, 0, 0) - #duration(1, 12, 0, 0),
        #duration(1, 0, 0, 0) < #duration(2, 0, 0, 0),
        #duration(1, 0, 0, 0) = #duration(0, 24, 0, 0)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
