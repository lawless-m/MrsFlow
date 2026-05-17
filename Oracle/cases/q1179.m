let
                fmt = BinaryFormat.Group({
                    BinaryFormat.Byte,
                    BinaryFormat.UnsignedInteger16
                }),
                r = try fmt(#binary({42, 1, 0})) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
