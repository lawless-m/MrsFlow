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
            bytes = Binary.ToList(rgba),
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
                    // Per-block-mean threshold: each sub-pixel is "on"
                    // if its luma is above the 2×2 block's mean.
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
            // Column names "C0", "C1", ... — PQ tables can't have
            // unnamed columns.
            colNames = List.Transform(xIndices, each "C" & Number.ToText(_))
        in
            Table.FromRows(rowsList, colNames)
in
    QuadrantTable
