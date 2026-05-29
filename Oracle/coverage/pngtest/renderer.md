# PQTI Reference Renderer: Pixel Buffer → Excel Workbook

**Audience:** the Claude instance building the renderer. New, parallel workstream — not blocked on the evaluator or M decoders.
**Author context:** produced in the same planning conversation as `PQTI_TEST_SUITE_PLAN.md`. Treat this as a design proposal to integrate; push back where wrong.

---

## TL;DR

A standalone Rust tool/library that takes a raw RGBA8 pixel buffer and produces an Excel workbook (`.xlsx`) where the image is rendered as a grid of Unicode quadrant block characters (U+2580–U+259F). Each character cell encodes a 2×2 pixel block. Optional per-cell foreground and background colour via conditional formatting gives two-colour-per-block fidelity — recognisable colour images at modest sizes.

**Why this exists as a separate workstream:**

- Pure function on raw pixels — no dependency on the evaluator, `BinaryFormat`, M, or the decoder.
- Can be built and validated entirely against synthetic test buffers.
- Once it works, it becomes a debugging aid for the decoder workstream the moment any pixels are coming out of an M decoder, even partial or wrong ones.
- Provides a "what should this test look like when it passes" companion artefact for every `.pqti` file.

**Integration:** `pqti-gen` (the test-file generator described in `PQTI_TEST_SUITE_PLAN.md` Part 3) calls this renderer to emit a `.xlsx` alongside each `.pqti` file.

---

## Encoding

### Glyph table (2×2 pixels per character)

For each non-overlapping 2×2 block of source pixels at position `(x, y)`:

1. Threshold each sub-pixel to on/off. Default rule: luminance `Y = 0.2126·R + 0.7152·G + 0.0722·B` > 0.5 → on. Pluggable threshold strategy (see below).
2. Pack the four on/off bits as `TL TR BL BR` → a 4-bit index 0..15.
3. Look up the glyph from this table:

| Bits (TL TR BL BR) | Glyph | Codepoint | Name                  |
|--------------------|-------|-----------|-----------------------|
| 0000               | ` `   | U+0020    | space                 |
| 0001               | `▗`   | U+2597    | lower right           |
| 0010               | `▖`   | U+2596    | lower left            |
| 0011               | `▄`   | U+2584    | lower half            |
| 0100               | `▝`   | U+259D    | upper right           |
| 0101               | `▐`   | U+2590    | right half            |
| 0110               | `▞`   | U+259E    | diagonal (TR + BL)    |
| 0111               | `▟`   | U+259F    | upper right + lower   |
| 1000               | `▘`   | U+2598    | upper left            |
| 1001               | `▚`   | U+259A    | diagonal (TL + BR)    |
| 1010               | `▌`   | U+258C    | left half             |
| 1011               | `▙`   | U+2599    | upper left + lower    |
| 1100               | `▀`   | U+2580    | upper half            |
| 1101               | `▜`   | U+259C    | upper left + right    |
| 1110               | `▛`   | U+259B    | upper left + lower L  |
| 1111               | `█`   | U+2588    | full block            |

This is the entire encoding surface for the basic two-tone mode. Sixteen glyphs, one table.

### Colour mode (optional, recommended)

When the source is RGB(A), enrich each cell with two colours:

- **Foreground colour:** mean RGB of the "on" sub-pixels in the 2×2 block (i.e. those above threshold). Applied as font colour.
- **Background colour:** mean RGB of the "off" sub-pixels. Applied as cell fill colour.

If all four sub-pixels are on, foreground is the mean of all four and background is left default (no fill). If all four are off, background is the mean of all four and the glyph is space.

This gives two-colour-per-2×2-block fidelity. A 32×32 image renders as a 16×16 grid of glyphs with up to two colours per cell — surprisingly recognisable, in the spirit of ZX Spectrum attribute blocks or DOS text-mode colour cells.

### Threshold strategy

Default: per-block local threshold (compare each sub-pixel against the mean luminance of its own 2×2 block). This handles low-contrast images better than a global threshold and avoids degenerate all-on or all-off blocks.

Pluggable alternatives worth implementing as options:
- Global 0.5 threshold (simple, predictable).
- Per-image Otsu's method (good for monochrome content).
- Alpha-channel threshold (when source has alpha and we want shape-not-luminance).

---

## Output workbook structure

One `.xlsx` per input buffer. Suggested sheet layout:

| Sheet name | Contents                                                                          |
|------------|-----------------------------------------------------------------------------------|
| `Image`    | The glyph table. One character per cell. Formatting applied per below.            |
| `Meta`     | Source dimensions, encoder version, threshold strategy used, source hash if any.  |

### Formatting rules for the `Image` sheet

- **Font:** Cascadia Mono preferred. Fallback chain: Cascadia Mono → Consolas → DejaVu Sans Mono → "any monospace font that renders U+2580–U+259F". Document this in `Meta` so a recipient with neither installed knows what to install.
- **Font size:** 14pt is a reasonable default. Larger for small images; smaller for big.
- **Row height and column width:** set so each cell is visually square. For 14pt Cascadia Mono, column width ≈ 2.0 (Excel units) and row height ≈ 16pt works on most systems. Compute these from font metrics if the underlying library exposes them; otherwise hardcode and document.
- **Cell alignment:** centre horizontal and vertical. Otherwise the glyphs sit awkwardly low in the cell.
- **Gridlines:** off. They visually segment the image.
- **Frozen panes:** none.
- **Per-cell font colour and fill colour** in colour mode, applied via direct formatting (not conditional formatting — direct is simpler and writes once).

### Library choice

In rough order of suitability for this job:

1. **`rust_xlsxwriter`** — pure Rust, actively maintained, handles font + fill colour + row/column dimensions cleanly. Recommended starting point.
2. **`umya-spreadsheet`** — also pure Rust, more featureful, but heavier and the API is less ergonomic for the write-only case we have here.
3. **`calamine`** — read-only, not suitable.

Confirm `rust_xlsxwriter` handles BMP-plane Unicode in cell strings correctly (it should — `.xlsx` is XML, characters travel as UTF-8). The quadrant block range is well-supported across all modern xlsx readers.

---

## API surface

A small library + a CLI wrapper.

### Library

```rust
pub struct RenderOptions {
    pub mode: RenderMode,                 // TwoTone | Colour
    pub threshold: ThresholdStrategy,     // PerBlockMean | Global(f32) | Otsu | Alpha
    pub font: String,                     // "Cascadia Mono"
    pub font_size: f64,                   // 14.0
    pub include_meta_sheet: bool,
}

pub struct PixelBuffer<'a> {
    pub width: u32,
    pub height: u32,
    pub rgba: &'a [u8],   // length must equal 4 * width * height
}

pub fn render_to_xlsx(
    buffer: &PixelBuffer,
    output_path: &Path,
    opts: &RenderOptions,
) -> Result<(), RenderError>;

pub fn render_to_workbook(
    buffer: &PixelBuffer,
    opts: &RenderOptions,
) -> Result<rust_xlsxwriter::Workbook, RenderError>;
```

The second form is for callers that want to embed multiple sheets in one workbook — e.g. a test report with one rendered image per failed test.

### CLI

```
pqti-render <input.rgba8> <width> <height> <output.xlsx> [--mode colour] [--threshold otsu]
```

Where `<input.rgba8>` is a raw byte file of length `4·width·height`. Trivial format for piping from anywhere.

---

## Validation plan

The renderer is independently testable without any decoder in the loop.

### Synthetic test inputs

A handful of hand-crafted RGBA8 buffers, generated in Rust, that exercise each encoding path:

1. **`solid_red`** — 8×8, all pixels (255, 0, 0, 255). Output: 4×4 grid of full-block glyphs, foreground red. Trivial sanity check.
2. **`black_and_white_checker`** — 8×8 single-pixel checkerboard. Output: 4×4 grid of `▞` and `▚` glyphs alternating. Tests the diagonal glyphs.
3. **`horizontal_gradient`** — 16×8, R goes 0 → 255 left-to-right, G=B=0. Tests threshold behaviour and that colour mode picks up the gradient.
4. **`vertical_stripes`** — alternating columns. Output: glyphs `▌` and `▐`.
5. **`pixel_text`** — a small bitmap font rendering of the word "PASS" in white on black. Should be human-readable in the output workbook. This is the goal-shaped test — if "PASS" reads as PASS, the renderer works.
6. **`alpha_mask`** — RGBA image with varying alpha. Tests the alpha-threshold strategy.

For each, store the expected output `.xlsx` as a checked-in artefact. Compare byte-for-byte on regeneration (xlsx files are deterministic given the same library version; if not, normalise via unzipping the xlsx and comparing the inner XML).

### Visual smoke test

A `cargo run --example gallery` that renders all the synthetic inputs to one workbook with one sheet each. Human opens it in Excel and confirms it looks right. Useful when adding new threshold strategies or tuning font defaults.

---

## Integration with `pqti-gen`

`pqti-gen` already computes the expected pixel buffer for each test file (it has to, in order to write the trailer hash). The integration is:

```rust
// In pqti-gen, after computing the expected buffer:
let buffer = run_pixel_program(&test_description);
let hash = sha256(&buffer);

// Write the .pqti binary
write_pqti_file(output_path, &test_description, &buffer, hash)?;

// Also write the .xlsx companion
let xlsx_path = output_path.with_extension("xlsx");
pqti_render::render_to_xlsx(&buffer.as_pixel_buffer(), &xlsx_path, &default_opts())?;
```

The xlsx companion is generated unconditionally for every test file. Disk cost is negligible — kilobytes per file — and it pays for itself the first time anyone opens one to ask "what's this test even supposed to look like?".

---

## Open questions

1. **Font metrics for square cells.** Excel's column width and row height units are font-dependent in ways that are awkward to compute exactly. We may need a small lookup table for "for font X at size Y, use column width W and row height H" rather than a formula. Worth investigating what `rust_xlsxwriter` exposes.
2. **Glyph variants.** The table above uses the canonical 16 quadrant compositions. Some fonts render these slightly differently (e.g. anti-aliasing the edges differently). Worth checking the rendering in Cascadia Mono, Consolas, and a couple of common cross-platform monospace fonts before freezing the choice.
3. **Larger images.** The 2×2-per-cell encoding handles up to ~128×128 source nicely (64×64 character grid). Beyond that the workbook becomes large and slow to open. Probably out of scope — PQTI test images are small by design — but worth noting. If we ever need larger, the 4×8 octant encoding (U+1FB00 onward, Symbols for Legacy Computing) gives 4× more spatial resolution per cell, at the cost of much patchier font support.
4. **Greyscale-only ramp mode.** As a third rendering mode alongside two-tone and colour, a 1-pixel-per-cell greyscale ramp using ` ░▒▓█` is sometimes more useful (e.g. dumping just an alpha channel). Cheap to add; worth including in the initial API or deferring?

---

## Suggested first milestone

A binary `pqti-render` that:

1. Reads a hardcoded `solid_red` 8×8 buffer.
2. Renders it to `out.xlsx` in two-tone mode.
3. Opens correctly in Excel and shows a red square of full-block glyphs.

From there, add the synthetic test inputs one at a time, then the colour mode, then integrate with `pqti-gen`.

---

## Non-goals

- This is not a general image-to-text-art tool. The encoding is tuned for small images that are meant to be exact reproductions, not artistic renderings of photographs.
- No support for `.xls` (the older binary format). `.xlsx` only.
- No interactive features (formulas, charts, named ranges). Pure formatted text output.
- No animation, no multi-frame output. One buffer in, one workbook out.