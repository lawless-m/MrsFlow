let
                fmt = BinaryFormat.Record([
                    a = BinaryFormat.Byte,
                    b = BinaryFormat.UnsignedInteger16,
                    c = BinaryFormat.UnsignedInteger16
                ]),
                r = try fmt(#binary({1, 2, 0, 100, 1})) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
