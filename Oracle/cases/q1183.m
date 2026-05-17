let
                fmt = BinaryFormat.Transform(BinaryFormat.Byte, (v) => v * 2),
                r = try fmt(#binary({21})) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
