let
                r = try {
                    BinaryFormat.Byte(#binary({42})),
                    BinaryFormat.UnsignedInteger16(#binary({1, 0})),
                    BinaryFormat.UnsignedInteger32(#binary({1, 0, 0, 0})),
                    BinaryFormat.UnsignedInteger16(#binary({255, 255})),
                    BinaryFormat.UnsignedInteger32(#binary({255, 255, 255, 255}))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
