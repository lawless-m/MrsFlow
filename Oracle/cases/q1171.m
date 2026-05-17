let
                r = try {
                    BinaryFormat.SignedInteger16(#binary({255, 255})),
                    BinaryFormat.SignedInteger32(#binary({0, 0, 0, 128})),
                    BinaryFormat.SignedInteger16(#binary({1, 0})),
                    BinaryFormat.SignedInteger32(#binary({255, 255, 255, 255}))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
