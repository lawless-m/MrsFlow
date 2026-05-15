// Shift counts past 63 bits.
let r = try {
        try Number.BitwiseShiftLeft(1, 64) otherwise "err",
        try Number.BitwiseShiftLeft(1, 65) otherwise "err",
        try Number.BitwiseShiftLeft(1, 100) otherwise "err",
        try Number.BitwiseShiftRight(1, 64) otherwise "err",
        try Number.BitwiseShiftLeft(1, -1) otherwise "err",
        try Number.BitwiseShiftRight(1, -1) otherwise "err",
        try Number.BitwiseShiftLeft(1, 0.5) otherwise "err"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
