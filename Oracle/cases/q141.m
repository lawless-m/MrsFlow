let
    r = try Value.Equals("Hello", "HELLO",
        (a,b) => Text.Lower(a) = Text.Lower(b))
in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
