// Json.Document deeply-nested arrays/records.
let r = try {
        Json.Document("{""a"": {""b"": {""c"": {""d"": 42}}}}"),
        Json.Document("[[[1, 2], [3, 4]], [[5, 6]]]"),
        Json.Document("{""arr"": [1, ""two"", null, true, [1, 2]]}")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
