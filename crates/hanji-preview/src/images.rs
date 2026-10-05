//! Image resource checks and bounded GIF first-frame normalization for PNG/JPEG output.
use std::{collections::BTreeMap, sync::Arc};

use oxml_layout::{LayoutResult, PositionedElement};
use resvg::usvg;

/// Limits on normalized RGBA image bytes, separate from encoded package bytes
/// and the output page raster. Each image occurrence is charged in layout order.
/// Repeated media are charged again; no assumption about browser cache reuse is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageLimits {
    /// Maximum width × height × 4 bytes for one image occurrence.
    pub max_image_decoded_bytes: u64,
    /// Sum of admitted image occurrences on one layout page.
    pub max_page_decoded_bytes: u64,
    /// Sum of admitted occurrences across all pages, including an HTML viewer.
    pub max_document_decoded_bytes: u64,
}

impl Default for ImageLimits {
    fn default() -> Self {
        // Match the existing worksheet's 16,777,216-pixel raster convention.
        let rgba_bytes = crate::xlsx::XlsxOptions::default().max_png_pixels * 4;
        Self {
            max_image_decoded_bytes: rgba_bytes,
            max_page_decoded_bytes: rgba_bytes,
            // Reuse the package reader's 1 GiB convention for the document total.
            max_document_decoded_bytes: hanji_package::package::MAX_UNPACKED,
        }
    }
}

impl ImageLimits {
    pub(crate) fn validate(self) -> Result<(), String> {
        if self.max_image_decoded_bytes == 0 || self.max_page_decoded_bytes == 0 || self.max_document_decoded_bytes == 0
        {
            return Err("embedded image budgets must be positive".into());
        }
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct ImagePlan {
    omitted: BTreeMap<String, String>,
    normalized: BTreeMap<String, (Vec<u8>, bool)>,
}

impl ImagePlan {
    pub(crate) fn inspect(layout: &LayoutResult, limits: ImageLimits) -> Result<Self, String> {
        limits.validate()?;
        let mut plan = Self::default();
        let mut document_bytes = 0;
        for (index, page) in layout.pages.iter().enumerate() {
            let mut page_bytes = 0;
            plan.inspect_elements(
                &page.elements,
                &format!("pages[{index}].elements"),
                limits,
                &mut page_bytes,
                &mut document_bytes,
            );
        }
        Ok(plan)
    }

    pub(crate) fn omission(&self, path: &str) -> Option<&str> {
        self.omitted.get(path).map(String::as_str)
    }

    pub(crate) fn normalized(&self, path: &str) -> Option<&[u8]> {
        self.normalized.get(path).map(|(png, _)| png.as_slice())
    }

    pub(crate) fn approximation(&self, path: &str) -> Option<&'static str> {
        self.normalized.get(path).and_then(|(_, animated)| animated.then_some(crate::quality::GIF_FIRST_FRAME))
    }

    fn inspect_elements(
        &mut self,
        elements: &[PositionedElement],
        parent: &str,
        limits: ImageLimits,
        page_bytes: &mut u64,
        document_bytes: &mut u64,
    ) {
        for (index, element) in elements.iter().enumerate() {
            let path = format!("{parent}[{index}]");
            match element {
                PositionedElement::Image { data, .. } => {
                    let Some(size) = decoded_bytes(data) else { continue };
                    let result = size.and_then(|bytes| {
                        if bytes > limits.max_image_decoded_bytes {
                            return Err("embedded image exceeds the configured decoded-image byte budget and was omitted");
                        }
                        if usize::try_from(bytes).is_err() || bytes > isize::MAX as u64 {
                            return Err("embedded image decoded bytes cannot be represented on this target and it was omitted");
                        }
                        let page_total = page_bytes.checked_add(bytes).ok_or(
                            "embedded image page decoded-byte total overflowed and the image was omitted",
                        )?;
                        let document_total = document_bytes.checked_add(bytes).ok_or(
                            "embedded image document decoded-byte total overflowed and the image was omitted",
                        )?;
                        if page_total > limits.max_page_decoded_bytes {
                            return Err("embedded image exceeds the configured page decoded-image byte budget and was omitted");
                        }
                        if document_total > limits.max_document_decoded_bytes {
                            return Err("embedded image exceeds the configured document decoded-image byte budget and was omitted");
                        }
                        let normalized = if is_gif(data) { Some(normalize_gif(data, bytes)?) } else { None };
                        *page_bytes = page_total;
                        *document_bytes = document_total;
                        Ok(normalized)
                    });
                    match result {
                        Ok(Some(png)) => {
                            self.normalized.insert(path, png);
                        }
                        Ok(None) => (),
                        Err(message) => {
                            self.omitted.insert(path, message.into());
                        }
                    }
                }
                PositionedElement::Group(group) => self.inspect_elements(
                    &group.children,
                    &format!("{path}.children"),
                    limits,
                    page_bytes,
                    document_bytes,
                ),
                PositionedElement::MarkedContent { children, .. } => {
                    self.inspect_elements(children, &format!("{path}.children"), limits, page_bytes, document_bytes)
                }
                _ => {}
            }
        }
    }
}

fn rgba_bytes(width: u32, height: u32) -> Result<u64, &'static str> {
    if width == 0 || height == 0 {
        return Err("embedded image dimensions must be positive and the image was omitted");
    }
    let bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("embedded image dimensions overflow the decoded RGBA byte count and the image was omitted")?;
    if width > i32::MAX as u32 / 4 || height > i32::MAX as u32 {
        return Err("embedded image dimensions exceed the raster backend representation and the image was omitted");
    }
    Ok(bytes)
}

fn decoded_bytes(data: &[u8]) -> Option<Result<u64, &'static str>> {
    let dimensions = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        // PNG has a fixed first IHDR. Read exact u32 dimensions, avoiding the
        // SVG metadata API's f32 rounding for very large declarations. CRC and
        // pixel validation remain the decoder's responsibility; no pixels are read.
        if data.get(8..16) != Some(b"\0\0\0\rIHDR") || data.len() < 33 {
            Err("embedded image PNG dimensions could not be read and the image was omitted")
        } else {
            Ok((
                u32::from_be_bytes(data[16..20].try_into().unwrap()),
                u32::from_be_bytes(data[20..24].try_into().unwrap()),
            ))
        }
    } else if is_gif(data) {
        if data.len() < 13 {
            Err("embedded image GIF dimensions could not be read and the image was omitted")
        } else {
            Ok((
                u32::from(u16::from_le_bytes(data[6..8].try_into().unwrap())),
                u32::from(u16::from_le_bytes(data[8..10].try_into().unwrap())),
            ))
        }
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        jpeg_dimensions(data)
    } else {
        // Preserve the existing unsupported-format diagnostic and behavior.
        return None;
    };
    Some(dimensions.and_then(|(width, height)| rgba_bytes(width, height)))
}

fn jpeg_dimensions(data: &[u8]) -> Result<(u32, u32), &'static str> {
    // Use the pinned SVG metadata parser, also used by the actual output path,
    // rather than adding a JPEG parser/dependency. The resolver supplies encoded
    // bytes directly: there is no base64 serialization or raster decoding here.
    let data = Arc::new(data.to_vec());
    let mut options = usvg::Options::default();
    options.image_href_resolver.resolve_string = Box::new(move |_, _| Some(usvg::ImageKind::JPEG(data.clone())));
    let tree = usvg::Tree::from_str(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><image href="embedded" width="1" height="1"/></svg>"#,
        &options,
    ).map_err(|_| "embedded image JPEG dimensions could not be read and the image was omitted")?;
    fn size(group: &usvg::Group) -> Option<(u32, u32)> {
        for node in group.children() {
            match node {
                usvg::Node::Image(image) => {
                    let width = image.size().width();
                    let height = image.size().height();
                    // JPEG SOF dimensions are u16, exactly representable by f32.
                    if width > 0.0
                        && height > 0.0
                        && width <= u16::MAX as f32
                        && height <= u16::MAX as f32
                        && width.fract() == 0.0
                        && height.fract() == 0.0
                    {
                        return Some((width as u32, height as u32));
                    }
                }
                usvg::Node::Group(group) => {
                    if let Some(size) = size(group) {
                        return Some(size);
                    }
                }
                _ => {}
            }
        }
        None
    }
    size(tree.root()).ok_or("embedded image JPEG dimensions could not be read and the image was omitted")
}

fn is_gif(data: &[u8]) -> bool {
    data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a")
}

fn normalize_gif(data: &[u8], bytes: u64) -> Result<(Vec<u8>, bool), &'static str> {
    let invalid = "embedded image GIF first frame could not be decoded and the image was omitted";
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    options.set_memory_limit(gif::MemoryLimit::Bytes(std::num::NonZeroU64::new(bytes).ok_or(invalid)?));
    let mut decoder = options.read_info(data).map_err(|_| invalid)?;
    let (width, height) = (decoder.width(), decoder.height());
    let frame = decoder.next_frame_info().map_err(|_| invalid)?.ok_or(invalid)?;
    // Offset/background compositing is outside this full-canvas first-frame slice.
    if frame.left != 0 || frame.top != 0 || frame.width != width || frame.height != height {
        return Err("embedded image GIF first frame does not cover its canvas and the image was omitted");
    }
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(bytes as usize).map_err(|_| invalid)?;
    pixels.resize(bytes as usize, 0);
    decoder.read_into_buffer(&mut pixels).map_err(|_| invalid)?;
    // Inspect only the next frame's metadata; never allocate or decode its pixels.
    let animated = decoder
        .next_frame_info()
        .map_err(|_| "embedded image GIF frame sequence could not be read and the image was omitted")?
        .is_some();
    // GIF alpha is binary. tiny-skia expects transparent pixels premultiplied.
    for pixel in pixels.as_chunks_mut::<4>().0 {
        if pixel[3] == 0 {
            pixel.fill(0);
        }
    }
    let size = resvg::tiny_skia::IntSize::from_wh(u32::from(width), u32::from(height)).ok_or(invalid)?;
    let png = resvg::tiny_skia::Pixmap::from_vec(pixels, size).ok_or(invalid)?.encode_png().map_err(|_| invalid)?;
    Ok((png, animated))
}
