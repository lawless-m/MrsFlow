// List.Sort with multi-key sort using composite key (record / list).
let xs = {
        [name="A", grade=2],
        [name="B", grade=1],
        [name="C", grade=2],
        [name="D", grade=1]
    } in
let r = try {
        List.Sort(xs, each {_[grade], _[name]})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
