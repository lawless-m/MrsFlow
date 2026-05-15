// Text.Format with record (named placeholders).
let r = try {
        Text.Format("hello #{name}", [name="world"]),
        Text.Format("#{a} + #{b}", [a=1, b=2]),
        Text.Format("#{0}", [a=1])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
