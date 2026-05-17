// Round-trip via Compression.GZip too.
            Binary.ToList(Binary.Decompress(
                Binary.Compress(#binary({5, 4, 3, 2, 1}), Compression.GZip),
                Compression.GZip))
