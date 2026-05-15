// IntegerDivide should agree with: a - (a mod b) all divided by b
// for both positive and negative operands. Smoke-test the relation.
let r = try {
        let
            a1 = 17, b1 = 5,
            a2 = -17, b2 = 5,
            a3 = 17, b3 = -5,
            a4 = -17, b4 = -5
        in {
            Number.IntegerDivide(a1, b1) = (a1 - Number.Mod(a1, b1)) / b1,
            Number.IntegerDivide(a2, b2) = (a2 - Number.Mod(a2, b2)) / b2,
            Number.IntegerDivide(a3, b3) = (a3 - Number.Mod(a3, b3)) / b3,
            Number.IntegerDivide(a4, b4) = (a4 - Number.Mod(a4, b4)) / b4
        }
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
