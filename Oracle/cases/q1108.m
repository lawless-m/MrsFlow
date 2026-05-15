// Text comparison with culture-sensitive characters (ordinal, not culture).
let r = try {
        "ß" < "ss",
        "ß" = "ß",
        "ı" < "i",
        "I" < "ı",
        "é" < "e",
        "café" = "café"
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
