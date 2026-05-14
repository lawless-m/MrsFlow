let r = try Splitter.SplitTextByCharacterTransition(
    {"a","b","c","d","e","f","g","h","i","j","k","l","m","n","o","p","q","r","s","t","u","v","w","x","y","z"},
    {"0","1","2","3","4","5","6","7","8","9"})("hello123world456") in
    if r[HasError]
        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
        else [HasError=false, Value=r[Value]]
