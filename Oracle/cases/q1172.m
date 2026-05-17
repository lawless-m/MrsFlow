let
                r = try {
                    BinaryFormat.Single(#binary({0, 0, 128, 63})),
                    BinaryFormat.Double(#binary({0, 0, 0, 0, 0, 0, 240, 63})),
                    BinaryFormat.Single(#binary({0, 0, 128, 191}))
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
