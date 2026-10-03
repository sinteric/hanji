//! Experimental HWPX page jobs. rhwp supplies geometry, images and controls;
//! Hanji supplies byte-based font selection, subsets, reports and raster output.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use base64::Engine as _;
use hanji_package::{package, xml};

use crate::fonts::{FontResolver, Fonts, Metrics, Script};
use crate::{
    subset, AsRequested, Diagnostic, FaceInfo, FontOptions, FontsReport, MissingGlyph, PageData, PageFormat, PageInfo,
    RenderedPage, Substituted,
};

struct Face {
    info: FaceInfo,
    data: Arc<[u8]>,
    bold: bool,
    italic: bool,
    chars: BTreeSet<char>,
    count: usize,
    pages: BTreeSet<usize>,
}

struct Page {
    svg: xml::Element,
    info: PageInfo,
    chars: BTreeMap<usize, BTreeSet<char>>,
    subsets: BTreeMap<usize, Arc<[u8]>>,
}

/// Independently owned document pages. No rhwp document or host font database
/// survives construction, and rendering never searches installed fonts.
pub struct Preview {
    pages: Vec<Page>,
    faces: Vec<Face>,
    document_subsets: BTreeMap<usize, Arc<[u8]>>,
    pub fonts: FontsReport,
    pub warnings: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Render HWPX using caller bytes, document fonts, then bundled fallbacks.
pub fn render_with_fonts(bytes: &[u8], options: &FontOptions) -> Result<Preview, String> {
    let (pages, embedded, diagnostics) = resolve(bytes)?;
    let fonts = Fonts::from_bytes(&options.fonts, &embedded, options.aliases.clone())?;
    finish(pages, diagnostics, &fonts)
}

#[cfg(all(feature = "host-fonts", not(target_family = "wasm")))]
pub fn render_native(bytes: &[u8], options: &crate::Options) -> Result<Preview, String> {
    let (pages, embedded, diagnostics) = resolve(bytes)?;
    let fonts = Fonts::load(&options.font_dirs, &embedded, options.system_fonts);
    finish(pages, diagnostics, &fonts)
}

type Resolved = (Vec<xml::Doc>, Vec<oxml_layout::FontFile>, Vec<Diagnostic>);

fn resolve(bytes: &[u8]) -> Result<Resolved, String> {
    let parts = package::read(bytes)?;
    if package::get(&parts, "Contents/header.xml").is_none() {
        return Err(
            "HWPX preview requires Contents/header.xml; HWP binary and HML inputs are not supported here".into()
        );
    }
    drop(parts);
    let document = rhwp::wasm_api::HwpDocument::from_bytes(bytes)
        .map_err(|e| format!("rhwp cannot open the HWPX document: {e}"))?;
    let mut pages = Vec::new();
    let mut diagnostics = Vec::new();
    let mut embedded = BTreeMap::new();
    for index in 0..document.page_count() {
        let svg = document
            .render_page_svg_layer_with_profile_native(index, rhwp::paint::RenderProfile::Screen)
            .map_err(|e| format!("rhwp cannot render page {}: {e}", index + 1))?;
        let mut tree =
            xml::parse(svg.as_bytes()).map_err(|e| format!("rhwp page {} produced invalid SVG: {e}", index + 1))?;
        tree.root.walk_mut(&mut |element| {
            if element.local() == "image" {
                for key in ["href", "xlink:href"] {
                    if element.get(key).is_some_and(|v| !v.starts_with("data:")) {
                        element.remove_attr(key);
                        diagnostics.push(Diagnostic {
                            path: format!("pages[{index}].images"),
                            message: "external image reference omitted; preview does not fetch external resources"
                                .into(),
                        });
                    }
                }
            }
            if element.local() == "style" {
                let css = text_content(element);
                for block in css.split("@font-face").skip(1) {
                    let Some(family) = declaration(block, "font-family") else {
                        continue;
                    };
                    let Some((_, encoded)) = block.split_once("base64,") else {
                        continue;
                    };
                    let encoded = encoded.split(['\'', '"', ')', ' ']).next().unwrap_or("");
                    if let Ok(data) = base64::engine::general_purpose::STANDARD.decode(encoded) {
                        embedded.entry(family.trim_matches(['\'', '"']).to_string()).or_insert(data);
                    }
                }
            }
        });
        let overflow = document.take_overflow_cell_lines();
        if overflow > 0 {
            diagnostics.push(Diagnostic {
                path: format!("pages[{index}].tables"),
                message: format!("rhwp reports {overflow} overflowing cell lines; table content may be clipped"),
            });
        }
        let overlaps = document.take_table_overlaps();
        if !overlaps.is_empty() {
            diagnostics.push(Diagnostic {
                path: format!("pages[{index}].tables"),
                message: format!("rhwp reports {} table overlaps", overlaps.len()),
            });
        }
        pages.push(tree);
    }
    let embedded = embedded.into_iter().map(|(family, data)| oxml_layout::FontFile { family, data }).collect();
    Ok((pages, embedded, diagnostics))
}

fn declaration<'a>(style: &'a str, key: &str) -> Option<&'a str> {
    let (_, value) = style.split_once(&format!("{key}:"))?;
    Some(value.split(';').next()?.trim())
}

fn text_content(element: &xml::Element) -> String {
    let mut text = String::new();
    for child in &element.children {
        match child {
            xml::Node::Text(value) => text.push_str(&xml::unescape(value)),
            xml::Node::CData(value) => text.push_str(value),
            xml::Node::El(child) if child.local() != "title" => text.push_str(&text_content(child)),
            _ => {}
        }
    }
    text
}

type FontRequest = (String, Script, bool, bool);

fn text_request(element: &xml::Element) -> Option<(FontRequest, String)> {
    if element.local() != "text" {
        return None;
    }
    let text = text_content(element);
    if text.is_empty() {
        return None;
    }
    let family = element.get("font-family").unwrap_or_else(|| "Arial".into());
    let requested = family.split(',').next().unwrap_or("Arial").trim().trim_matches(['\'', '"']).to_string();
    let bold = element.get("font-weight").is_some_and(|v| v == "bold" || v.parse::<u16>().is_ok_and(|n| n >= 600));
    let italic = element.get("font-style").is_some_and(|v| matches!(v.as_str(), "italic" | "oblique"));
    Some(((requested, Script::of(&text), bold, italic), text))
}

fn finish(
    trees: Vec<xml::Doc>,
    mut diagnostics: Vec<Diagnostic>,
    resolver: &dyn FontResolver,
) -> Result<Preview, String> {
    let mut used_chars: BTreeMap<FontRequest, BTreeSet<char>> = BTreeMap::new();
    for tree in &trees {
        tree.root.walk(&mut |element| {
            if let Some((key, text)) = text_request(element) {
                used_chars.entry(key).or_default().extend(text.chars());
            }
        });
    }
    let mut faces: Vec<Face> = Vec::new();
    let mut ids = BTreeMap::new();
    let mut pages = Vec::new();
    let mut missing: BTreeMap<char, (String, BTreeSet<usize>)> = BTreeMap::new();
    for (index, mut tree) in trees.into_iter().enumerate() {
        let mut chars: BTreeMap<usize, BTreeSet<char>> = BTreeMap::new();
        let mut error = None;
        tree.root.walk_mut(&mut |element| {
            if error.is_some() {
                return;
            }
            let Some((key, text)) = text_request(element) else {
                return;
            };
            let (requested, script, bold, italic) = key.clone();
            let id = if let Some(id) = ids.get(&key) {
                *id
            } else {
                let needed: String = used_chars[&key].iter().collect();
                let resolved = resolver.resolve_font_for_text(&requested, script, bold, italic, &needed);
                let draws_script = resolved.is_some();
                let Some(font) = resolved.or_else(|| resolver.resolve_font(&requested, Script::Latin, bold, italic))
                else {
                    error = Some(format!("no usable font for HWPX text requesting {requested}"));
                    return;
                };
                let id = faces.len();
                let parsed = match ttf_parser::Face::parse(&font.font.data, font.font.face_index) {
                    Ok(face) => face,
                    Err(e) => {
                        error = Some(format!("font {requested}: {e}"));
                        return;
                    }
                };
                let style = (parsed.is_bold(), parsed.is_italic());
                let data = match crate::sfnt::build(
                    &font.font.data,
                    font.font.face_index,
                    &crate::sfnt::Adjust {
                        family: format!("hanji-font-{id}"),
                        bold: style.0,
                        italic: style.1,
                        line_em: None,
                        ea_advance: None,
                    },
                ) {
                    Ok(data) => data,
                    Err(e) => {
                        error = Some(format!("font {requested}: {e}"));
                        return;
                    }
                };
                faces.push(Face {
                    info: FaceInfo {
                        requested: requested.clone(),
                        script,
                        drawn: draws_script.then_some(font.family),
                        source: draws_script.then_some(font.source),
                        // rhwp retains source/table geometry rather than laying out with the selected drawing font.
                        metrics: if draws_script && font.metrics == Metrics::Original {
                            Metrics::Original
                        } else {
                            Metrics::Substitute
                        },
                    },
                    data: Arc::from(data),
                    bold: style.0,
                    italic: style.1,
                    chars: BTreeSet::new(),
                    count: 0,
                    pages: BTreeSet::new(),
                });
                ids.insert(key, id);
                id
            };
            let face = &mut faces[id];
            face.count += text.chars().filter(|c| !c.is_whitespace()).count();
            face.pages.insert(index + 1);
            face.chars.extend(text.chars());
            chars.entry(id).or_default().extend(text.chars());
            if let Ok(parsed) = ttf_parser::Face::parse(&face.data, 0) {
                if crate::fonts::last_resort(&parsed) {
                    diagnostics.push(Diagnostic {
                        path: format!("pages[{index}].fonts[{id}]"),
                        message: crate::quality::GENERIC_FONT.into(),
                    });
                }
                for c in text.chars().filter(|c| !c.is_whitespace()) {
                    if crate::fonts::last_resort(&parsed) || parsed.glyph_index(c).is_none_or(|g| g.0 == 0) {
                        missing.entry(c).or_insert_with(|| (requested.clone(), BTreeSet::new())).1.insert(index + 1);
                    }
                }
            }
            element.set("font-family", &format!("hanji-font-{id}"));
            element.set("data-font-requested", &face.info.requested);
            element.set("data-font-drawn", face.info.drawn.as_deref().unwrap_or(""));
            if face.info.substituted() {
                element.set("data-font-substituted", "true");
                let mut title = xml::Element::new("title");
                title.children.push(xml::Node::Text(xml::escape_text(&face.info.tooltip())));
                element.children.push(xml::Node::El(title));
            }
        });
        if let Some(error) = error {
            return Err(error);
        }
        let size = tree.root.get("viewBox").unwrap_or_default();
        let values: Vec<f64> = size.split_whitespace().filter_map(|v| v.parse().ok()).collect();
        if values.len() != 4 || !values[2].is_finite() || !values[3].is_finite() || values[2] <= 0.0 || values[3] <= 0.0
        {
            return Err(format!("HWPX page {} has an invalid SVG viewBox", index + 1));
        }
        pages.push(Page {
            svg: tree.root,
            info: PageInfo { index, width: values[2] * 0.75, height: values[3] * 0.75 },
            chars,
            subsets: BTreeMap::new(),
        });
    }
    let mut fonts = FontsReport::default();
    for face in &faces {
        if face.info.substituted() {
            fonts.substituted.push(Substituted {
                requested: face.info.requested.clone(),
                script: face.info.script,
                drawn: face.info.drawn.clone(),
                source: face.info.source,
                metrics: face.info.metrics.clone(),
                chars: face.count,
                pages: face.pages.iter().copied().collect(),
            });
        } else if let Some(source) = face.info.source {
            fonts.drawn_as_requested.push(AsRequested {
                requested: face.info.requested.clone(),
                script: face.info.script,
                source,
                chars: face.count,
            });
        }
    }
    fonts.missing_glyphs_total = missing.len();
    fonts.missing_glyphs = missing
        .into_iter()
        .take(crate::MISSING_LISTED)
        .map(|(c, (requested, pages))| MissingGlyph {
            char: format!("U+{:04X}", c as u32),
            requested,
            pages: pages.into_iter().collect(),
        })
        .collect();
    let mut document_subsets = BTreeMap::new();
    for (id, face) in faces.iter().enumerate() {
        for page in &mut pages {
            if let Some(chars) = page.chars.get(&id) {
                match subset::subset(&face.data, 0, chars) {
                    Ok(data) => {
                        page.subsets.insert(id, Arc::from(data));
                    }
                    Err(e) => diagnostics.push(Diagnostic {
                        path: format!("pages[{}].fonts[{id}].subset", page.info.index),
                        message: e,
                    }),
                }
            }
        }
        match subset::subset(&face.data, 0, &face.chars) {
            Ok(data) => {
                document_subsets.insert(id, Arc::from(data));
            }
            Err(e) => diagnostics.push(Diagnostic { path: format!("viewer.fonts[{id}].subset"), message: e }),
        }
    }
    let mut warnings = resolver.warnings().to_vec();
    warnings.push("HWPX preview is experimental: rhwp uses source/table font geometry; drawing substitutions can differ from Hancom, and complex controls or table reflow need visual review.".into());
    if faces.iter().any(|f| f.info.script == Script::Hangul && f.info.drawn.is_none()) {
        warnings.push(crate::NO_KOREAN_FONT.into());
    }
    warnings.extend(diagnostics.iter().map(|d| format!("{}: {}", d.path, d.message)));
    Ok(Preview { pages, faces, document_subsets, fonts, warnings, diagnostics })
}

/// Scope local CSS IRIs without changing colors, external/data URLs, comments
/// or quoted CSS text. Prefixing unresolved fragments also prevents a missing
/// definition from accidentally binding to another page or the viewer itself.
fn scoped_urls(value: &str, prefix: &str) -> String {
    fn skip_string(bytes: &[u8], start: usize) -> usize {
        let quote = bytes[start];
        let mut i = start + 1;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' => i = (i + 2).min(bytes.len()),
                c if c == quote => return i + 1,
                _ => i += 1,
            }
        }
        i
    }
    let bytes = value.as_bytes();
    let (mut i, mut copied) = (0, 0);
    let mut out = String::new();
    while i < bytes.len() {
        if matches!(bytes[i], b'\'' | b'"') {
            i = skip_string(bytes, i);
            continue;
        }
        if bytes[i..].starts_with(b"/*") {
            i = value[i + 2..].find("*/").map_or(bytes.len(), |end| i + 2 + end + 2);
            continue;
        }
        let boundary = i == 0
            || !(bytes[i - 1].is_ascii_alphanumeric()
                || matches!(bytes[i - 1], b'_' | b'-' | b'\\')
                || bytes[i - 1] >= 128);
        if boundary && bytes.get(i..i + 3).is_some_and(|s| s.eq_ignore_ascii_case(b"url")) {
            let mut start = i + 3;
            while bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
                start += 1;
            }
            if bytes.get(start) == Some(&b'(') {
                start += 1;
                let mut end = start;
                while end < bytes.len() && bytes[end] != b')' {
                    match bytes[end] {
                        b'\'' | b'"' => end = skip_string(bytes, end),
                        b'\\' => end = (end + 2).min(bytes.len()),
                        _ => end += 1,
                    }
                }
                if end < bytes.len() {
                    let target = value[start..end].trim();
                    let target = if target.starts_with(['\'', '"'])
                        && target.ends_with(target.chars().next().unwrap())
                        && target.len() >= 2
                    {
                        &target[1..target.len() - 1]
                    } else {
                        target
                    };
                    if target.starts_with('#') && target.len() > 1 {
                        let fragment = start + value[start..end].find('#').unwrap() + 1;
                        out.push_str(&value[copied..fragment]);
                        out.push_str(prefix);
                        copied = fragment;
                    }
                    i = end + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    out.push_str(&value[copied..]);
    out
}

fn viewer_svg(page: &xml::Element, index: usize) -> String {
    let prefix = format!("hanji-hwpx-p{}-", index + 1);
    let mut root = page.clone();
    root.walk_mut(&mut |e| {
        for (key, raw) in &mut e.attrs {
            let value = xml::unescape(raw);
            let scoped = match xml::local_name(key) {
                "id" => format!("{prefix}{value}"),
                "href" => {
                    let target = value.trim();
                    if target.starts_with('#') && target.len() > 1 {
                        let position = value.find('#').unwrap() + 1;
                        format!("{}{prefix}{}", &value[..position], &value[position..])
                    } else {
                        value
                    }
                }
                "aria-labelledby"
                | "aria-describedby"
                | "aria-controls"
                | "aria-owns"
                | "aria-flowto"
                | "aria-activedescendant"
                | "aria-details"
                | "aria-errormessage" => {
                    value.split_whitespace().map(|id| format!("{prefix}{id}")).collect::<Vec<_>>().join(" ")
                }
                "style" | "clip-path" | "mask" | "filter" | "fill" | "stroke" | "cursor" | "marker"
                | "marker-start" | "marker-mid" | "marker-end" | "color-profile" | "shape-inside"
                | "shape-subtract" | "background" | "background-image" => scoped_urls(&value, &prefix),
                _ => value,
            };
            *raw = xml::escape_attr(&scoped);
        }
        if e.local() == "style" {
            for node in &mut e.children {
                match node {
                    xml::Node::Text(value) => *value = xml::escape_text(&scoped_urls(&xml::unescape(value), &prefix)),
                    xml::Node::CData(value) => *value = scoped_urls(value, &prefix),
                    _ => {}
                }
            }
        }
    });
    root.to_xml()
}

impl Preview {
    pub fn quality(&self) -> crate::quality::QualityReport {
        crate::quality::QualityReport::inspect(
            crate::quality::PreviewSource::Hwpx,
            &self.diagnostics,
            &self.fonts,
            &self.warnings,
        )
    }
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
    pub fn page_info(&self, index: usize) -> Option<PageInfo> {
        self.pages.get(index).map(|p| p.info)
    }

    fn css(&self, subsets: &BTreeMap<usize, Arc<[u8]>>, prefix: &str) -> String {
        subsets
            .iter()
            .map(|(&id, data)| {
                subset::font_face(id as u32, data, self.faces[id].bold, self.faces[id].italic)
                    .replace("hanji-font-", prefix)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn svg(&self, index: usize) -> Result<String, String> {
        let page = self.pages.get(index).ok_or_else(|| format!("page index {index} is out of range"))?;
        let prefix = format!("hanji-page-{index}-font-");
        let mut tree = page.svg.clone();
        tree.walk_mut(&mut |e| {
            if let Some(family) = e.get("font-family").filter(|f| f.starts_with("hanji-font-")) {
                e.set("font-family", &family.replacen("hanji-font-", &prefix, 1));
            }
        });
        let svg = tree.to_xml();
        let position = svg.find('>').ok_or("SVG root is absent")? + 1;
        Ok(format!("{}<style>{}</style>{}", &svg[..position], self.css(&page.subsets, &prefix), &svg[position..]))
    }

    pub fn render_page(&self, index: usize, format: PageFormat) -> Result<RenderedPage, String> {
        let page = self.pages.get(index).ok_or_else(|| format!("page index {index} is out of range"))?;
        let data = match format {
            PageFormat::Svg => PageData::Svg(self.svg(index)?),
            PageFormat::Png { dpi } => {
                if !dpi.is_finite() || dpi <= 0.0 {
                    return Err("PNG DPI must be finite and positive".into());
                }
                let mut options = resvg::usvg::Options::default();
                let mut ids = BTreeMap::new();
                for (&id, data) in &page.subsets {
                    let loaded = options
                        .fontdb_mut()
                        .load_font_source(resvg::usvg::fontdb::Source::Binary(Arc::new(data.clone())));
                    if let Some(&loaded) = loaded.first() {
                        ids.insert(format!("hanji-font-{id}"), loaded);
                    }
                }
                options.font_resolver = resvg::usvg::FontResolver {
                    select_font: Box::new(move |font, _| {
                        font.families().iter().find_map(|f| ids.get(f.to_string().trim_matches('"')).copied())
                    }),
                    select_fallback: Box::new(|_, _, _| None),
                };
                let tree =
                    resvg::usvg::Tree::from_str(&page.svg.to_xml(), &options).map_err(|e| format!("usvg: {e}"))?;
                let scale = dpi / 96.0;
                let (width, height) =
                    ((tree.size().width() as f64 * scale).round(), (tree.size().height() as f64 * scale).round());
                if width < 1.0 || height < 1.0 || width > u32::MAX as f64 || height > u32::MAX as f64 {
                    return Err("HWPX page dimensions cannot be rasterized at this DPI".into());
                }
                let mut pixels = resvg::tiny_skia::Pixmap::new(width as u32, height as u32)
                    .ok_or("HWPX page is too large to rasterize")?;
                pixels.fill(resvg::tiny_skia::Color::WHITE);
                resvg::render(
                    &tree,
                    resvg::tiny_skia::Transform::from_scale(scale as f32, scale as f32),
                    &mut pixels.as_mut(),
                );
                PageData::Png(pixels.encode_png().map_err(|e| format!("PNG: {e}"))?)
            }
        };
        let prefix = format!("pages[{index}].");
        Ok(RenderedPage {
            page: page.info,
            data,
            diagnostics: self
                .diagnostics
                .iter()
                .filter(|d| !d.path.starts_with("pages[") || d.path.starts_with(&prefix))
                .cloned()
                .collect(),
        })
    }

    pub fn html(&self, title: &str) -> String {
        let esc = xml::escape_text;
        let warnings = self
            .warnings
            .iter()
            .chain(self.fonts.lines().iter())
            .map(|s| format!("<li>{}</li>", esc(s)))
            .collect::<String>();
        let pages = self
            .pages
            .iter()
            .enumerate()
            .map(|(i, p)| format!("<section id=\"page-{}\">{}</section>", i + 1, viewer_svg(&p.svg, i)))
            .collect::<String>();
        format!("<!doctype html><html lang=\"ko\"><meta charset=\"utf-8\"><title>{}</title><style>{}\nbody{{background:#eee;font:14px sans-serif}}header{{background:white;padding:12px}}section{{margin:20px auto;background:white;width:fit-content}}svg{{display:block;max-width:100%;height:auto}}[data-font-substituted=true]{{text-decoration:underline dotted}}</style><header><h1>{}</h1><ul>{warnings}</ul></header><main>{pages}</main></html>", esc(title), self.css(&self.document_subsets, "hanji-font-"), esc(title))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewer_scopes_local_svg_and_css_references_without_changing_other_values() {
        let input = br##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 100 100">
          <defs><clipPath id="clip"/><clipPath id="clip-long"/><mask id="mask"/><filter id="filter"/>
            <linearGradient id="paint"/><linearGradient id="paint-copy" xlink:href="#paint"/>
            <marker id="arrow"/><path id="fff"/><title id="title&amp;label">label</title><desc id="description">description</desc></defs>
          <style><![CDATA[.note { clip-path: URL( '#clip' ); mask: url( #mask ); fill: url("#paint"); stroke: #fff;
            content: "url(#clip)"; cursor: url(https://example.invalid/vector.svg#paint); }
            /* url(#clip) */ @font-face { src: url('data:font/ttf;base64,AAAA'); }]]></style>
          <g id="shape" clip-path="url(#clip-long)" mask="url('#mask')" filter="url( &quot;#filter&quot; )"
            fill="url(#paint)" stroke="#fff" marker-end="url(#arrow)"
            aria-labelledby="title&amp;label description" data-font-requested="url(#clip)"
            style="clip-path:url(#clip);fill:url(#missing);color:#fff">
            <use href="#fff"/><use xlink:href="#paint"/><a href="https://example.invalid/#paint"/>
          </g></svg>"##;
        let page = xml::parse(input).unwrap().root;
        let before = page.to_xml();
        for index in [0, 1] {
            let prefix = format!("hanji-hwpx-p{}-", index + 1);
            let scoped = xml::parse(viewer_svg(&page, index).as_bytes()).unwrap().root;
            let defs = scoped.child("defs").unwrap();
            assert!(defs.elements().all(|e| e.get("id").unwrap().starts_with(&prefix)));
            assert_eq!(defs.child("linearGradient").unwrap().get("id"), Some(format!("{prefix}paint")));
            let gradient = defs.elements().find(|e| e.get("id") == Some(format!("{prefix}paint-copy"))).unwrap();
            assert_eq!(gradient.get("xlink:href"), Some(format!("#{prefix}paint")));
            let group = scoped.child("g").unwrap();
            assert_eq!(group.get("clip-path"), Some(format!("url(#{prefix}clip-long)")));
            assert_eq!(group.get("mask"), Some(format!("url('#{prefix}mask')")));
            assert_eq!(group.get("filter"), Some(format!("url( \"#{prefix}filter\" )")));
            assert_eq!(group.get("fill"), Some(format!("url(#{prefix}paint)")));
            assert_eq!(group.get("stroke"), Some("#fff".into()));
            assert_eq!(group.get("marker-end"), Some(format!("url(#{prefix}arrow)")));
            assert_eq!(group.get("aria-labelledby"), Some(format!("{prefix}title&label {prefix}description")));
            assert_eq!(group.get("data-font-requested"), Some("url(#clip)".into()));
            assert_eq!(
                group.get("style"),
                Some(format!("clip-path:url(#{prefix}clip);fill:url(#{prefix}missing);color:#fff"))
            );
            let uses: Vec<_> = group.elements().filter(|e| e.local() == "use").collect();
            assert_eq!(uses[0].get("href"), Some(format!("#{prefix}fff")));
            assert_eq!(uses[1].get("xlink:href"), Some(format!("#{prefix}paint")));
            assert_eq!(group.child("a").unwrap().get("href"), Some("https://example.invalid/#paint".into()));
            let css = text_content(scoped.child("style").unwrap());
            assert!(css.contains(&format!("URL( '#{prefix}clip' )")));
            assert!(css.contains(&format!("mask: url( #{prefix}mask )")));
            assert!(css.contains(&format!("fill: url(\"#{prefix}paint\")")));
            assert!(css.contains("stroke: #fff") && css.contains("content: \"url(#clip)\""));
            assert!(css.contains("/* url(#clip) */") && css.contains("https://example.invalid/vector.svg#paint"));
            assert!(css.contains("data:font/ttf;base64,AAAA"));
        }
        assert_eq!(page.to_xml(), before, "viewer serialization leaves standalone page data unchanged");
    }

    #[test]
    fn a_sparse_latin_face_does_not_hide_later_digits() {
        let original = oxml_layout::bundled_fonts::bundled_font_data()[0].1;
        let sparse = subset::subset(original, 0, &BTreeSet::from(['A'])).unwrap();
        let sparse = crate::sfnt::build(
            &sparse,
            0,
            &crate::sfnt::Adjust {
                family: "Sparse Test".into(),
                bold: false,
                italic: false,
                line_em: None,
                ea_advance: None,
            },
        )
        .unwrap();
        let options = FontOptions { fonts: vec![crate::FontData::new(sparse)], ..Default::default() };
        let fonts = Fonts::from_bytes(&options.fonts, &[], None).unwrap();
        let tree = xml::parse(br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><text x="5" y="20" font-family="Sparse Test">A</text><text x="5" y="40" font-family="Sparse Test">123</text></svg>"#).unwrap();
        let preview = finish(vec![tree], vec![], &fonts).unwrap();
        assert_eq!(preview.fonts.missing_glyphs_total, 0);
        assert!(preview
            .fonts
            .substituted
            .iter()
            .any(|f| f.requested == "Sparse Test" && f.drawn.as_deref() == Some("Liberation Sans") && f.chars == 4));
        let PageData::Svg(svg) = preview.render_page(0, PageFormat::Svg).unwrap().data else { panic!("SVG") };
        assert!(xml::parse(svg.as_bytes()).unwrap().root.text_of(&["text", "tspan"]).contains("123"));
        assert!(svg.contains("data:font/"));
        let PageData::Png(png) = preview.render_page(0, PageFormat::Png { dpi: 96.0 }).unwrap().data else {
            panic!("PNG")
        };
        assert!(png.starts_with(b"\x89PNG"));
    }
}
