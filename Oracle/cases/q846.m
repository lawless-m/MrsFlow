// Text.Reverse identity — twice = original.
let r = try {
        Text.Reverse(Text.Reverse("hello")) = "hello",
        Text.Reverse(Text.Reverse("café")) = "café",
        Text.Reverse(Text.Reverse("")) = "",
        Text.Length(Text.Reverse("hello world")) = Text.Length("hello world")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
