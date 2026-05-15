// List.Sort stability — equal keys should preserve input order.
// Tag elements with a record then sort by k; the tag should appear in
// input order within each k-group.
let xs = {
        [k=1, tag="A"],
        [k=2, tag="B"],
        [k=1, tag="C"],
        [k=2, tag="D"],
        [k=1, tag="E"]
    } in
let r = try {
        List.Sort(xs, (a, b) => Value.Compare(a[k], b[k])),
        List.Sort(xs, each _[k])
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
