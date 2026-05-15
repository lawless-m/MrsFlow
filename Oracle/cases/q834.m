// Text.Format unterminated placeholder.
let r = try {
        Text.Format("#{0", {"X"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
