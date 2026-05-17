let
                fmt = BinaryFormat.Length(BinaryFormat.Text(3), BinaryFormat.Byte),
                r = try fmt(#binary({3, 97, 98, 99})) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
