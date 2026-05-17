let
                fmt = BinaryFormat.Text(5),
                r = try fmt(#binary({72, 101, 108, 108, 111})) in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
