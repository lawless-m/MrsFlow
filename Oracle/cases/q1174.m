let
                fmt = BinaryFormat.Binary(3),
                r = try Binary.ToText(fmt(#binary({1, 2, 3, 4, 5})), BinaryEncoding.Hex) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
