// Compress then decompress should round-trip.
            Binary.ToList(Binary.Decompress(
                Binary.Compress(#binary({0, 1, 2, 3, 4, 5}), Compression.Deflate),
                Compression.Deflate))
