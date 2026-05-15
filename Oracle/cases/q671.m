let r = try {
        Number.IntegerDivide(7.5, 2),
        Number.IntegerDivide(-7.5, 2),
        Number.IntegerDivide(7.5, -2),
        Number.IntegerDivide(-7.5, -2),
        Number.IntegerDivide(7, 2.5),
        Number.IntegerDivide(-7, 2.5),
        Number.IntegerDivide(0.5, 0.25),
        Number.IntegerDivide(-0.5, 0.25)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
