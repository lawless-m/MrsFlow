let
                v = Binary.View(#binary({1,2,3}), [GetLength = () => 3])
            in
                Binary.Length(v)
