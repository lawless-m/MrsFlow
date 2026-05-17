// BinaryFormat.UnsignedInteger64 — also 0 for zero bytes.
            let
                fmt = BinaryFormat.UnsignedInteger64,
                r = try fmt(#binary({0,0,0,0,0,0,0,0}))
            in
                if r[HasError] then "ERR" else Number.ToText(r[Value])
