let
                leFmt = BinaryFormat.UnsignedInteger32,
                beFmt = BinaryFormat.ByteOrder(BinaryFormat.UnsignedInteger32, ByteOrder.BigEndian),
                bs = #binary({0, 0, 0, 1}),
                r = try { leFmt(bs), beFmt(bs) } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
