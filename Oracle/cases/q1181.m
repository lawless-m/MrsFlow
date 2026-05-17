let
                fmt = BinaryFormat.Choice(BinaryFormat.Byte, (key) =>
                    if key = 1 then BinaryFormat.UnsignedInteger16
                    else if key = 2 then BinaryFormat.UnsignedInteger32
                    else BinaryFormat.Null),
                r = try {
                    fmt(#binary({1, 100, 0})),
                    fmt(#binary({2, 1, 0, 0, 0})),
                    fmt(#binary({9}))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
