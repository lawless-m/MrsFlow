// Boundaries: dividing by -1, by 1, very small divisors.
let r = try {
        Number.IntegerDivide(100, -1),
        Number.IntegerDivide(-100, -1),
        Number.IntegerDivide(100, 1),
        Number.IntegerDivide(-100, 1),
        Number.IntegerDivide(1, 100),
        Number.IntegerDivide(-1, 100),
        Number.IntegerDivide(99, 100),
        Number.IntegerDivide(-99, 100)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
