// PngToQuadrants — given a PNG file path, return a Power Query table
// where each cell is a Unicode quadrant glyph representing a 2×2 block
// of the source image.
//
// To use in Power Query / Excel:
//   1. Open the Advanced Editor on a new blank query.
//   2. Paste the contents of this file.
//   3. Replace the example path on the last `let` line with your PNG.
//   4. Refresh. Apply Cascadia Mono / Consolas to the resulting cells
//      so the glyphs render correctly.
//
// In mrsflow (CLI / WASM): same source, just runs via mrsflow.exe.

let
    // ============================================================
    // PNG decoder (greyscale 8-bit, all five filter types)
    // ============================================================
    PngDecode = (input as binary) as record =>
        let
            sigBytes = Binary.ToList(Binary.Range(input, 0, 8)),
            expectedSig = {137, 80, 78, 71, 13, 10, 26, 10},
            sigOk = List.Zip({sigBytes, expectedSig}),
            sigMatch = List.Sum(List.Transform(sigOk, each
                if _{0} = _{1} then 0 else 1)) = 0,

            chunkAt = (bin as binary, offset as number) =>
                let
                    rest = Binary.Range(bin, offset),
                    len = BinaryFormat.ByteOrder(BinaryFormat.UnsignedInteger32, ByteOrder.BigEndian)(rest),
                    typeBytes = Binary.Range(rest, 4, 4),
                    typeText = Text.FromBinary(typeBytes, TextEncoding.Utf8),
                    data = Binary.Range(rest, 8, len),
                    consumed = 12 + len
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

            ihdrChunk = List.First(List.Select(chunks, each _[Type] = "IHDR"), null),
            ihdr = if ihdrChunk = null then null else
                let
                    d = ihdrChunk[Data],
                    width  = BinaryFormat.ByteOrder(BinaryFormat.UnsignedInteger32, ByteOrder.BigEndian)(Binary.Range(d, 0, 4)),
                    height = BinaryFormat.ByteOrder(BinaryFormat.UnsignedInteger32, ByteOrder.BigEndian)(Binary.Range(d, 4, 4)),
                    bitDepth   = BinaryFormat.Byte(Binary.Range(d, 8, 1)),
                    colorType  = BinaryFormat.Byte(Binary.Range(d, 9, 1)),
                    interlace  = BinaryFormat.Byte(Binary.Range(d, 12, 1))
                in
                    [ Width = width, Height = height, BitDepth = bitDepth,
                      ColorType = colorType, Interlace = interlace ],

            idatChunks = List.Select(chunks, each _[Type] = "IDAT"),
            idatBytes = Binary.Combine(List.Transform(idatChunks, each _[Data])),
            deflateInner = if idatBytes = null then null
                else Binary.Range(idatBytes, 2, Binary.Length(idatBytes) - 6),
            decompressed = if deflateInner = null then null
                else Binary.Decompress(deflateInner, Compression.Deflate),

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
                    unfilter = (rowIdx as number, priorRow as list) as list =>
                        let
                            base = rowIdx * rowStride,
                            filterType = Number.From(Binary.ToList(Binary.Range(decomp, base, 1)){0}),
                            filt = Binary.ToList(Binary.Range(decomp, base + 1, width)),
                            indices = List.Numbers(0, width),
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
                                        else error "PngDecode: unknown filter type"
                                in
                                    acc & {recon})
                        in
                            result,
                    zerosRow = List.Repeat({0}, width),
                    rowIndices = List.Numbers(0, height),
                    rowState = List.Accumulate(rowIndices, [Prior = zerosRow, Rows = {}], (state, idx) =>
                        let row = unfilter(idx, state[Prior])
                        in [ Prior = row, Rows = state[Rows] & {row} ]),
                    allSamples = List.Combine(rowState[Rows]),
                    rgbaList = List.Combine(List.Transform(allSamples, each {_, _, _, 255}))
                in
                    Binary.FromList(rgbaList),

            rgbaBuffer = if ihdr = null or decompressed = null then null
                else if ihdr[ColorType] <> 0 then null
                else if ihdr[BitDepth] <> 8 then null
                else if ihdr[Interlace] <> 0 then null
                else decodeGray8(decompressed, ihdr[Width], ihdr[Height])
        in
            if not sigMatch then
                [ Success = false, Error = "invalid PNG signature" ]
            else if ihdr = null then
                [ Success = false, Error = "no IHDR" ]
            else if rgbaBuffer = null then
                [ Success = false, Error = "stage 1 supports only greyscale 8-bit non-interlaced" ]
            else
                [ Success = true,
                  Width = ihdr[Width], Height = ihdr[Height],
                  RGBA8 = rgbaBuffer, Error = "" ],

    // ============================================================
    // Quadrant-glyph renderer
    // ============================================================
    glyphs = {
        " ", "#(2597)", "#(2596)", "#(2584)",
        "#(259D)", "#(2590)", "#(259E)", "#(259F)",
        "#(2598)", "#(259A)", "#(258C)", "#(2599)",
        "#(2580)", "#(259C)", "#(259B)", "#(2588)"
    },

    QuadrantTable = (rgba as binary, width as number, height as number) as table =>
        let
            bytes = Binary.ToList(rgba),
            lumaAt = (idx as number) =>
                let r = bytes{idx * 4}, g = bytes{idx * 4 + 1}, b = bytes{idx * 4 + 2}
                in 0.2126 * r + 0.7152 * g + 0.0722 * b,
            pxIdx = (x, y) => if x >= width or y >= height then null else y * width + x,
            cellGlyph = (cx, cy) =>
                let
                    x = cx * 2, y = cy * 2,
                    iTL = pxIdx(x,     y),     iTR = pxIdx(x + 1, y),
                    iBL = pxIdx(x,     y + 1), iBR = pxIdx(x + 1, y + 1),
                    lTL = if iTL = null then 0 else lumaAt(iTL),
                    lTR = if iTR = null then 0 else lumaAt(iTR),
                    lBL = if iBL = null then 0 else lumaAt(iBL),
                    lBR = if iBR = null then 0 else lumaAt(iBR),
                    mean = (lTL + lTR + lBL + lBR) / 4,
                    onTL = if lTL > mean then 1 else 0,
                    onTR = if lTR > mean then 1 else 0,
                    onBL = if lBL > mean then 1 else 0,
                    onBR = if lBR > mean then 1 else 0,
                    idx = onTL * 8 + onTR * 4 + onBL * 2 + onBR
                in
                    glyphs{idx},
            cols = Number.RoundUp(width / 2),
            rows = Number.RoundUp(height / 2),
            xIndices = List.Numbers(0, cols),
            yIndices = List.Numbers(0, rows),
            rowsList = List.Transform(yIndices, (cy) =>
                List.Transform(xIndices, (cx) => cellGlyph(cx, cy))),
            colNames = List.Transform(xIndices, each "C" & Number.ToText(_))
        in
            Table.FromRows(rowsList, colNames),

    // ============================================================
    // Entry point — change the path here
    // ============================================================
    PngPath = "c:/Users/matthew.heath/Git/MrsFlow/tools/png-decoder/png-suite/basn0g08.png",
    decoded = PngDecode(File.Contents(PngPath))
in
    if decoded[Success]
        then QuadrantTable(decoded[RGBA8], decoded[Width], decoded[Height])
        else error decoded[Error]
