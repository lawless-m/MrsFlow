// Split into 2-byte chunks.
            List.Transform(Binary.Split(#binary({1, 2, 3, 4, 5}), 2),
                each Binary.ToList(_))
