let
                r = try {
                    BinaryFormat.7BitEncodedUnsignedInteger(#binary({5})),
                    BinaryFormat.7BitEncodedUnsignedInteger(#binary({172, 2})),
                    BinaryFormat.7BitEncodedUnsignedInteger(#binary({128, 1})),
                    BinaryFormat.7BitEncodedSignedInteger(#binary({5})),
                    BinaryFormat.7BitEncodedSignedInteger(#binary({172, 2}))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
