// Text.Trim with unicode chars in trim set.
let r = try {
        Text.Trim("→abc←", {"→", "←"}),
        Text.Trim("→→abc←←", {"→", "←"}),
        Text.Trim("éabcé", {"é"}),
        Text.Trim("ßabcß", {"ß"})
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
