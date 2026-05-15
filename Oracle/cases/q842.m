// Text.Reverse basic — ASCII, empty, single char, palindrome.
let r = try {
        Text.Reverse(""),
        Text.Reverse("a"),
        Text.Reverse("ab"),
        Text.Reverse("abc"),
        Text.Reverse("hello"),
        Text.Reverse("aba"),
        Text.Reverse("racecar")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
