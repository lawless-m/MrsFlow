let r = try
        let
            orig = Binary.FromList({170, 187, 204, 221}),
            hex = Binary.ToText(orig, BinaryEncoding.Hex),
            roundtrip = Binary.FromText(hex, BinaryEncoding.Hex),
            equal = Binary.ToText(roundtrip, BinaryEncoding.Base64) = Binary.ToText(orig, BinaryEncoding.Base64)
        in
            equal
    in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
