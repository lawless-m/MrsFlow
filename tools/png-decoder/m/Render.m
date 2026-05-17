// Render an RGBA8 buffer as a Power Query table of Unicode quadrant
// glyphs. Each non-overlapping 2×2 source block becomes one cell,
// drawn with a single character from U+2580–U+259F + space + full
// block (U+2588).
//
// Usage:
//   let
//       decoderSrc = Text.FromBinary(File.Contents("...Decode.m"), TextEncoding.Utf8),
//       PngDecode = Expression.Evaluate(decoderSrc, #shared),
//       rendererSrc = Text.FromBinary(File.Contents("...Render.m"), TextEncoding.Utf8),
//       QuadrantTable = Expression.Evaluate(rendererSrc, #shared),
//       decoded = PngDecode(File.Contents("c:/path/to/img.png"))
//   in
//       QuadrantTable(decoded[RGBA8], decoded[Width], decoded[Height])
//
// Result: a Table with `ceil(width/2)` columns and `ceil(height/2)` rows.
// Each cell is a one-character text value. The Excel user can then set
// the Image-sheet font to Cascadia Mono / Consolas / any monospace
// font that renders the quadrant block range.
//
// Colour: glyph-only in v1. Adding per-cell fg/bg colour from M alone
// is awkward — PQ output is data, not formatting — so we leave that to
// the user's Excel conditional-formatting rules. A future revision
// could return a sibling table of hex colour codes.

let
    // 16 glyphs indexed by (tl<<3) | (tr<<2) | (bl<<1) | br
    glyphs = {
        " ",           // 0000
        "#(2597)",     // 0001  ▗
        "#(2596)",     // 0010  ▖
        "#(2584)",     // 0011  ▄
        "#(259D)",     // 0100  ▝
        "#(2590)",     // 0101  ▐
        "#(259E)",     // 0110  ▞
        "#(259F)",     // 0111  ▟
        "#(2598)",     // 1000  ▘
        "#(259A)",     // 1001  ▚
        "#(258C)",     // 1010  ▌
        "#(2599)",     // 1011  ▙
        "#(2580)",     // 1100  ▀
        "#(259C)",     // 1101  ▜
        "#(259B)",     // 1110  ▛
        "#(2588)"      // 1111  █
    },

    QuadrantTable = (rgba as binary, width as number, height as number) as table =>
        let
            // List.Buffer hints PQ to materialise as an array-backed
            // list so `bytes{idx}` is O(1). Without it, indexing into a
            // 230K-element list (240×240×4 RGBA) walks the list — every
            // cell pays O(N) per pixel lookup, so a 240×240 render that
            // *should* take seconds takes 10+ minutes.
            bytes = List.Buffer(Binary.ToList(rgba)),
            // Luma at flat index — Rec. 709 weights.
            lumaAt = (idx as number) =>
                let
                    r = bytes{idx * 4},
                    g = bytes{idx * 4 + 1},
                    b = bytes{idx * 4 + 2}
                in
                    0.2126 * r + 0.7152 * g + 0.0722 * b,
            // Pixel idx for (x, y), or null if out of bounds.
            pxIdx = (x as number, y as number) =>
                if x >= width or y >= height then null
                else y * width + x,
            // Otsu threshold — pick the 0..255 cut that maximises inter-
            // class variance over the 8-bit luminance histogram. One
            // global threshold for the whole image; the per-block-mean
            // alternative produces noise (by construction half each
            // block is below its own mean regardless of image content).
            pixelCount = width * height,
            lumas = List.Transform(List.Numbers(0, pixelCount), (i) =>
                Number.RoundDown(lumaAt(i))),
            clamped = List.Transform(lumas, (v) =>
                if v < 0 then 0 else if v > 255 then 255 else v),
            // Sort + single-pass run-length count → record keyed by
            // luminance string. O(N log N) for the sort, O(N) for the
            // pass — far better than O(N*256) (256 List.Count(List.Select)
            // sweeps) or O(N*256) (ReplaceRange Accumulate).
            sorted = List.Sort(clamped),
            // Walk the sorted list, accumulating runs. State carries the
            // current value, its running count, and the completed buckets
            // (Vals + Counts parallel lists).
            // List.Buffer on the Vals/Cnts carries each step is critical
            // for Excel — without it, PQ keeps `Vals` and `Cnts` as a
            // chain of lazy references through every prior accumulator
            // record. The chain grows to N pixels deep and field access
            // walks it. Buffering forces each step to materialise its own
            // list. mrsflow's iterative force handles either shape, but
            // the eager form is also faster there.
            runState = List.Accumulate(sorted,
                [Cur = -1, Cnt = 0, Vals = {}, Cnts = {}],
                (s, v) =>
                    if v = s[Cur] then
                        [Cur = s[Cur], Cnt = s[Cnt] + 1,
                         Vals = List.Buffer(s[Vals]),
                         Cnts = List.Buffer(s[Cnts])]
                    else if s[Cur] = -1 then
                        [Cur = v, Cnt = 1,
                         Vals = List.Buffer(s[Vals]),
                         Cnts = List.Buffer(s[Cnts])]
                    else
                        [Cur = v, Cnt = 1,
                         Vals = List.Buffer(s[Vals] & {Number.ToText(s[Cur])}),
                         Cnts = List.Buffer(s[Cnts] & {s[Cnt]})]),
            // Flush the final run.
            finalKeys = if runState[Cur] = -1 then runState[Vals]
                else runState[Vals] & {Number.ToText(runState[Cur])},
            finalCnts = if runState[Cur] = -1 then runState[Cnts]
                else runState[Cnts] & {runState[Cnt]},
            grouped = Record.FromList(finalCnts, finalKeys),
            hist = List.Transform(List.Numbers(0, 256), (i) =>
                let key = Number.ToText(i)
                in if Record.HasFields(grouped, {key}) then Record.Field(grouped, key) else 0),
            sumAll = List.Sum(List.Transform(List.Numbers(0, 256),
                each _ * hist{_})),
            otsuStep = (state, i) =>
                let
                    wB = state[WB] + hist{i},
                    sumB = state[SumB] + i * hist{i}
                in
                    if wB = 0 or wB = pixelCount
                        then [WB = wB, SumB = sumB, Best = state[Best], BestT = state[BestT]]
                        else
                            let
                                wF = pixelCount - wB,
                                mB = sumB / wB,
                                mF = (sumAll - sumB) / wF,
                                between = wB * wF * (mB - mF) * (mB - mF)
                            in
                                if between > state[Best]
                                    then [WB = wB, SumB = sumB, Best = between, BestT = i]
                                    else [WB = wB, SumB = sumB, Best = state[Best], BestT = state[BestT]],
            otsuState = List.Accumulate(List.Numbers(0, 256),
                [WB = 0, SumB = 0, Best = 0, BestT = 0], otsuStep),
            threshold = otsuState[BestT],
            cellGlyph = (cx as number, cy as number) =>
                let
                    x = cx * 2,
                    y = cy * 2,
                    iTL = pxIdx(x,     y),
                    iTR = pxIdx(x + 1, y),
                    iBL = pxIdx(x,     y + 1),
                    iBR = pxIdx(x + 1, y + 1),
                    lTL = if iTL = null then 0 else lumaAt(iTL),
                    lTR = if iTR = null then 0 else lumaAt(iTR),
                    lBL = if iBL = null then 0 else lumaAt(iBL),
                    lBR = if iBR = null then 0 else lumaAt(iBR),
                    onTL = if lTL > threshold then 1 else 0,
                    onTR = if lTR > threshold then 1 else 0,
                    onBL = if lBL > threshold then 1 else 0,
                    onBR = if lBR > threshold then 1 else 0,
                    idx = onTL * 8 + onTR * 4 + onBL * 2 + onBR
                in
                    glyphs{idx},
            cols = Number.RoundUp(width / 2),
            rows = Number.RoundUp(height / 2),
            xIndices = List.Numbers(0, cols),
            yIndices = List.Numbers(0, rows),
            rowsList = List.Transform(yIndices, (cy) =>
                List.Transform(xIndices, (cx) => cellGlyph(cx, cy))),
            // Column names "C0", "C1", ... — PQ tables can't have
            // unnamed columns.
            colNames = List.Transform(xIndices, each "C" & Number.ToText(_))
        in
            Table.FromRows(rowsList, colNames)
in
    QuadrantTable
