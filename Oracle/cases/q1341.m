// BinaryFormat.Decimal — 16-byte decimal, all-zero input
            // decodes to 0.
            let
                fmt = BinaryFormat.Decimal,
                r = try fmt(#binary({0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0}))
            in
                if r[HasError] then "ERR" else Number.ToText(r[Value])
