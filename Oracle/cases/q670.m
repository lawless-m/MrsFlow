let r = try {
        Number.IntegerDivide(10, 3),
        Number.IntegerDivide(-10, 3),
        Number.IntegerDivide(10, -3),
        Number.IntegerDivide(-10, -3),
        Number.IntegerDivide(0, 5),
        Number.IntegerDivide(0, -5),
        Number.IntegerDivide(3, 3),
        Number.IntegerDivide(-3, 3)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
