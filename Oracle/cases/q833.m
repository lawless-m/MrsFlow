// Text.Format error cases — placeholder index out of range, unterminated, etc.
let r = try {
        Text.Format("#{0}", {})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
