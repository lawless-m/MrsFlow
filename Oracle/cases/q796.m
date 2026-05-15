// Text.Insert unicode — character-counted offset.
let r = try {
        Text.Insert("café", 3, "X"),
        Text.Insert("café", 4, "X"),
        Text.Insert("→→", 1, "←"),
        Text.Insert("naïve", 2, "→")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
