// List.Range with offset > length or negative.
let xs = {10, 20, 30} in
let r = try {
        List.Range(xs, 10)
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
