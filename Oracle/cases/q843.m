// Text.Reverse with BMP non-ASCII chars.
let r = try {
        Text.Reverse("café"),
        Text.Reverse("→←"),
        Text.Reverse("hello world"),
        Text.Reverse("naïve"),
        Text.Reverse("ß")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
