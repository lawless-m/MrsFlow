// All characters are hex digits or dashes.
let g = Text.NewGuid() in
let allValidChars =
    List.AllTrue(
        List.Transform(
            Text.ToList(g),
            (c) => List.Contains({"0","1","2","3","4","5","6","7","8","9","a","b","c","d","e","f","-"}, c)
        )
    ) in
let r = try {
        allValidChars
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
