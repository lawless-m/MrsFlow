let
                fmt = BinaryFormat.List(BinaryFormat.Byte, 3),
                r = try fmt(#binary({10, 20, 30, 40, 50})) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
