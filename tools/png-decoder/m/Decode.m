// PNG decoder for mrsflow — stage 1.
//
// Scope: grayscale, 8-bit per sample, non-interlaced, filter type 0
// (None) only. Validates the PNG signature, parses chunks via
// BinaryFormat, extracts IHDR, concatenates IDAT, zlib-strips, deflates
// via Binary.Decompress, splits filtered scanlines, and emits a
// row-major RGBA8 byte buffer with each grey sample replicated to R, G,
// B and alpha set to 255.
//
// Public entry point: `PngDecode = (input as binary) as record => ...`
// Result:
//   [
//     Success      = logical,
//     Width        = number,    // when Success
//     Height       = number,    // when Success
//     BitDepth     = number,    // when Success
//     ColorType    = number,    // when Success
//     RGBA8        = binary,    // width*height*4 bytes, when Success
//     Error        = text       // when not Success
//   ]
//
// Not implemented in stage 1 (errors clearly):
//   - color types other than 0 (greyscale)
//   - bit depths other than 8
//   - filter types other than 0 (None)
//   - Adam7 interlacing
//   - CRC32 / Adler32 verification (diagnostic; pixel-hash mismatch
//     catches integrity failures anyway)

let
    PngDecode = (input as binary) as record =>
        let
            // --- Signature check (8 bytes) ---
            sigBytes = Binary.ToList(Binary.Range(input, 0, 8)),
            expectedSig = {137, 80, 78, 71, 13, 10, 26, 10},
            sigOk = List.Zip({sigBytes, expectedSig}),
            sigMatch = List.Sum(List.Transform(sigOk, each
                if _{0} = _{1} then 0 else 1)) = 0,

            // --- Chunk walk: read chunks until we hit IEND ---
            // Each chunk: length(u32 BE) | type(4 ascii bytes) | data(length) | crc(u32 BE)
            chunkAt = (bin as binary, offset as number) =>
                let
                    rest = Binary.Range(bin, offset),
                    len = BinaryFormat.ByteOrder(BinaryFormat.UnsignedInteger32, ByteOrder.BigEndian)(rest),
                    typeBytes = Binary.Range(rest, 4, 4),
                    typeText = Text.FromBinary(typeBytes, TextEncoding.Utf8),
                    data = Binary.Range(rest, 8, len),
                    consumed = 12 + len  // 4 length + 4 type + len data + 4 crc
                in
                    [ Type = typeText, Data = data, Consumed = consumed ],

            walkChunks = (bin as binary, offset as number, acc as list) as list =>
                let
                    c = chunkAt(bin, offset),
                    acc2 = acc & {c}
                in
                    if c[Type] = "IEND" then acc2
                    else @walkChunks(bin, offset + c[Consumed], acc2),

            chunks = if sigMatch then walkChunks(input, 8, {}) else {},

            // --- IHDR ---
            ihdrChunk = List.First(List.Select(chunks, each _[Type] = "IHDR"), null),
            ihdr = if ihdrChunk = null then null else
                let
                    d = ihdrChunk[Data],
                    width  = BinaryFormat.ByteOrder(BinaryFormat.UnsignedInteger32, ByteOrder.BigEndian)(Binary.Range(d, 0, 4)),
                    height = BinaryFormat.ByteOrder(BinaryFormat.UnsignedInteger32, ByteOrder.BigEndian)(Binary.Range(d, 4, 4)),
                    bitDepth   = BinaryFormat.Byte(Binary.Range(d, 8, 1)),
                    colorType  = BinaryFormat.Byte(Binary.Range(d, 9, 1)),
                    compressionM = BinaryFormat.Byte(Binary.Range(d, 10, 1)),
                    filterMethod = BinaryFormat.Byte(Binary.Range(d, 11, 1)),
                    interlace    = BinaryFormat.Byte(Binary.Range(d, 12, 1))
                in
                    [ Width = width, Height = height, BitDepth = bitDepth,
                      ColorType = colorType, Compression = compressionM,
                      FilterMethod = filterMethod, Interlace = interlace ],

            // --- IDAT concatenation + zlib strip + deflate ---
            idatChunks = List.Select(chunks, each _[Type] = "IDAT"),
            idatBytes = Binary.Combine(List.Transform(idatChunks, each _[Data])),

            // zlib wrapper around the deflate stream: 2-byte header
            // (CMF + FLG) and a 4-byte Adler32 trailer. Strip both;
            // pass the middle to Binary.Decompress(_, Compression.Deflate).
            deflateInner = if idatBytes = null then null
                else Binary.Range(idatBytes, 2, Binary.Length(idatBytes) - 6),

            decompressed = if deflateInner = null then null
                else Binary.Decompress(deflateInner, Compression.Deflate),

            // --- Greyscale 8-bit, all five PNG filter types ---
            // Each scanline = 1 filter byte + width sample bytes.
            // Filter types per PNG spec RFC 2083 §6:
            //   0 None / 1 Sub / 2 Up / 3 Average / 4 Paeth.
            // bpp = 1 for greyscale 8-bit; all arithmetic mod 256.
            //
            // Uses List.Accumulate for the inner-column and outer-row
            // loops because mrsflow's evaluator doesn't TCO recursive
            // M closures — straight recursion overflows the stack on
            // any non-trivial image.
            decodeGray8 = (decomp as binary, width as number, height as number) =>
                let
                    bpp = 1,
                    rowStride = width + 1,
                    paeth = (a, b, c) =>
                        let p = a + b - c,
                            pa = Number.Abs(p - a),
                            pb = Number.Abs(p - b),
                            pc = Number.Abs(p - c)
                        in
                            if pa <= pb and pa <= pc then a
                            else if pb <= pc then b
                            else c,
                    // Unfilter a single scanline. priorRow is a list of
                    // `width` recon bytes for the previous row (zeros
                    // for the first row).
                    unfilter = (rowIdx as number, priorRow as list) as list =>
                        let
                            base = rowIdx * rowStride,
                            filterType = Number.From(Binary.ToList(Binary.Range(decomp, base, 1)){0}),
                            filt = Binary.ToList(Binary.Range(decomp, base + 1, width)),
                            indices = List.Numbers(0, width),
                            // Accumulate the recon row left-to-right.
                            result = List.Accumulate(indices, {}, (acc, i) =>
                                let
                                    a = if i - bpp >= 0 then acc{i - bpp} else 0,
                                    b = priorRow{i},
                                    c = if i - bpp >= 0 then priorRow{i - bpp} else 0,
                                    f = filt{i},
                                    recon =
                                        if filterType = 0 then f
                                        else if filterType = 1 then Number.Mod(f + a, 256)
                                        else if filterType = 2 then Number.Mod(f + b, 256)
                                        else if filterType = 3 then Number.Mod(f + Number.RoundDown((a + b) / 2), 256)
                                        else if filterType = 4 then Number.Mod(f + paeth(a, b, c), 256)
                                        else error "PngDecode: unknown filter type " & Number.ToText(filterType)
                                in
                                    acc & {recon})
                        in
                            result,
                    zerosRow = List.Repeat({0}, width),
                    rowIndices = List.Numbers(0, height),
                    // State carried through the row fold: [prior, allRows].
                    rowState = List.Accumulate(rowIndices, [Prior = zerosRow, Rows = {}], (state, idx) =>
                        let
                            row = unfilter(idx, state[Prior])
                        in
                            [ Prior = row, Rows = state[Rows] & {row} ]),
                    allRows = rowState[Rows],
                    allSamples = List.Combine(allRows),
                    rgbaList = List.Combine(List.Transform(allSamples, each {_, _, _, 255}))
                in
                    Binary.FromList(rgbaList),

            rgbaBuffer = if ihdr = null or decompressed = null then null
                else if ihdr[ColorType] <> 0 then null
                else if ihdr[BitDepth] <> 8 then null
                else if ihdr[Interlace] <> 0 then null
                else decodeGray8(decompressed, ihdr[Width], ihdr[Height]),

            failure = (msg as text) as record =>
                [ Success = false, Error = msg, Width = 0, Height = 0,
                  BitDepth = 0, ColorType = 0, RGBA8 = #binary({}) ]
        in
            if not sigMatch then
                failure("PngDecode: invalid PNG signature")
            else if ihdr = null then
                failure("PngDecode: no IHDR chunk found")
            else if ihdr[ColorType] <> 0 then
                failure("PngDecode stage 1: only color type 0 (greyscale) supported; got " & Number.ToText(ihdr[ColorType]))
            else if ihdr[BitDepth] <> 8 then
                failure("PngDecode stage 1: only 8-bit depth supported; got " & Number.ToText(ihdr[BitDepth]))
            else if ihdr[Interlace] <> 0 then
                failure("PngDecode stage 1: non-interlaced only; got Interlace=" & Number.ToText(ihdr[Interlace]))
            else if rgbaBuffer = null then
                failure("PngDecode: pipeline produced no buffer (unknown error)")
            else
                [ Success = true,
                  Width = ihdr[Width],
                  Height = ihdr[Height],
                  BitDepth = ihdr[BitDepth],
                  ColorType = ihdr[ColorType],
                  RGBA8 = rgbaBuffer,
                  Error = "" ]
in
    PngDecode
