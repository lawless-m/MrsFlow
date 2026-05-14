let r = try error [Reason="Custom.Reason", Message="msg-here", Detail="details"] in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
