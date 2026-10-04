# Embedded image budgets

Shared PPTX/DOCX previews preflight embedded PNG/JPEG images before the shared
output stage decodes image pixels or embeds original image bytes in SVG/HTML.
The exact same admission plan applies to all three encodings. This closes a
DOCX output gap: tiny PNG/JPEG payloads can declare a 1 GiB output buffer while
the old shared lowerer embeds them without a resource diagnostic. HWPX uses a
separate renderer; these limits do not configure its image pipeline. XLSX
currently omits worksheet images and retains its existing window/package/raster
budgets.

`ImageLimits::default()` allows **64 MiB of normalized RGBA bytes per image and
per page**, and **1 GiB across the document**. The first two defaults reuse the
worksheet preview's 16,777,216-pixel output convention (four bytes per pixel);
the document total reuses the package reader's 1 GiB convention. These are
resource budgets, not page-count or image-format expansion limits.

The estimate is `width × height × 4`, using positive integer dimensions and
checked `u64` multiplication/addition. PNG's fixed first IHDR provides exact
`u32` dimensions, including declarations that cannot be represented exactly by
SVG's `f32` metadata. JPEG uses the pinned SVG metadata parser; JPEG dimensions
are `u16` and exactly representable by that API. JPEG's raster decoder rejects
multiple SOF headers rather than replacing its dimensions. Metadata inspection
does not decompress pixels. It does not replace codec validation of CRCs, pixel
data, or unsupported JPEG variants.

Rust allocation representability and the raster backend's dimension bounds
also apply; configuration cannot disable these arithmetic/representation checks.
The estimate counts normalized pixel storage, not exact process memory: codec
working buffers, effects, encoded bytes, fonts and the output page pixmap are
additional resources. Lowering output DPI does not reduce source-image decoding
requirements.

Images are considered in page/element order, including nested groups and marked
content. Each occurrence is charged, including repeated media, without relying
on browser cache deduplication. The page sum resets on every page; the document
sum carries across pages, including the HTML viewer. A refused image does not
consume the remaining allowance, so a smaller later image can still fit.

An over-budget image, invalid dimension declaration, or unrepresentable count is
omitted with a diagnostic such as:

```json
{
  "path": "pages[1].elements[3]",
  "message": "embedded image exceeds the configured page decoded-image byte budget and was omitted"
}
```

Warnings and HTML expose the omission before output is chosen. The versioned
quality report classifies resource refusals as `svg.image-resource-limit` with
error severity and an omission consequence. Existing critical-loss strictness
can refuse the preview before an adapter writes output. Malformed dimension
metadata uses the existing `svg.image-omitted` category.

Existing entry points retain their signatures and use these defaults. New
constructors allow a caller to choose its own positive budgets before preflight:

```rust
use hanji_preview::{render_pptx_with_fonts_and_limits, FontOptions, ImageLimits};

let limits = ImageLimits {
    max_image_decoded_bytes: 64 << 20,
    max_page_decoded_bytes: 128 << 20,
    max_document_decoded_bytes: 1 << 30,
};
let preview = render_pptx_with_fonts_and_limits(bytes, &FontOptions::default(), limits)?;
```

DOCX exposes `docx::render_with_fonts_and_limits`. Native font adapters expose
`render_pptx_with_limits` and `docx::render_native_with_limits`; a custom font
resolver can use `render_pptx_with_resolver_and_limits`. `Preview::image_limits`
returns the selected policy. No required fields were added to `FontOptions` or
native `Options`, and no codec dependencies were added.

PPTX already has upstream limits of 16 MiB encoded/64 MiB decoded per image
before its compatibility raster check. They remain in force. The new shared
preflight does not reconfigure or remove those earlier checks; tighter caller
budgets govern Hanji output after them, and higher budgets cannot revive an
image the engine already refused. Aggregate/page admission supplements that
existing per-image guard. The current engine also requires three-channel
8-bit JPEG for its compatibility check; that restriction is unchanged.

Ordinary admitted images retain their original bytes and output behavior.
Compatibility changes are deliberate for images beyond these budgets or with
unreadable dimensions: they now have visible omissions rather than unbounded
allocation attempts or silent decoder failures. Clients with larger trusted
documents can raise explicit budgets. GIF/WebP and animation behavior remain
unchanged; this policy does not enable additional formats.

Owned regression fixtures cover ordinary red PNG/gray JPEG pixels, tiny payloads
declaring 1 GiB output, exact PNG dimension arithmetic, configuration, nested
occurrences, aggregate budgets and public PPTX/DOCX imports. The large metadata
fixtures must never be rasterized before this preflight is applied.
