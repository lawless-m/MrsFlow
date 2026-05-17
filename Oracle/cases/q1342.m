// BinaryFormat.SignedInteger64 — 0 bytes = 0.
            let
                fmt = BinaryFormat.SignedInteger64,
                r = try fmt(#binary({0,0,0,0,0,0,0,0}))
            in
                if r[HasError] then "ERR" else Number.ToText(r[Value])
