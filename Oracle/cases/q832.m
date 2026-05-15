// Text.Format with various value types — number, date, null, list, record.
let r = try {
        Text.Format("n=#{0}", {42}),
        Text.Format("n=#{0}", {3.14}),
        Text.Format("n=#{0}", {null}),
        Text.Format("n=#{0}", {true}),
        Text.Format("n=#{0}", {"text"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
