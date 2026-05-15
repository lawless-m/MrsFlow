// Text.Reverse with combining marks — does PQ reverse codepoints or graphemes?
// "é" can be precomposed (U+00E9) or e+combining acute (U+0065 U+0301).
// "café" with decomposed é = "café" — reversing codepoints would
// move the combining mark away from the e.
let r = try {
        Text.Reverse("cafe#(0301)"),
        Text.Length("cafe#(0301)"),
        Text.Reverse("a#(0301)b#(0301)"),
        Text.Reverse("a#(0301)")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
