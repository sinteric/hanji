//! hanji's preview (DESIGN.md §2 rules 4–5, §7, §7.1): renders the package
//! an export writes, one SVG per slide, with the fonts each slide draws
//! subset and embedded, an HTML viewer that marks substituted text, and PNG.
//! pptx only, through rpptx 0.12.1; docx and hwpx come later.
//!
//! The pipeline: the export's bytes → the engine-compat transforms of
//! [`prep`] on a copy → rpptx resolves the slides → every run's font
//! request (the typeface rpptx would pick, and the script of its text) is
//! resolved by [`fonts`] and handed to rpptx's layout as a face of its own
//! ([`sfnt`]: renamed, 1.2 em lines) → the layout is lowered to SVG
//! ([`svg`], from rdocx) → the fonts report is read off the layout.
//!
//! Native only (it reads font files); on wasm32 the crate is empty.

#![cfg(not(target_family = "wasm"))]

pub mod fonts;
pub mod prep;
pub mod sfnt;
pub mod store;
pub mod subset;
mod svg;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use oxml_layout::{FontFile, FontId, LayoutResult, PositionedElement};
use rpptx_layout::{ResolvedBullet, ResolvedContent, ResolvedRunStyle, ResolvedTextBody, ResolvedTextRun};
use serde::Serialize;

use fonts::{Drawn, Fonts, Metrics, Script, Source};

/// Where the preview finds fonts.
#[derive(Clone, Debug)]
pub struct Options {
    /// Font files and directories, searched first (`--font-dir`, `HANJI_FONT_DIR`).
    pub font_dirs: Vec<PathBuf>,
    /// Whether installed system fonts are searched (after the font directories).
    pub system_fonts: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options { font_dirs: vec![], system_fonts: true }
    }
}

impl Options {
    /// `font_dirs` (`--font-dir`), then `HANJI_FONT_DIR`, then system fonts.
    pub fn with_font_dirs(mut font_dirs: Vec<PathBuf>) -> Options {
        font_dirs.extend(fonts::env_font_dirs());
        Options { font_dirs, system_fonts: true }
    }
}

/// A run's font as rpptx picks it (`rpptx-render` 0.12.1 `typeface_for_text`):
/// the symbol, East Asian, complex-script or Latin typeface by the text,
/// then any other. rpptx draws a run with none in Arial.
fn requested_typeface(style: &ResolvedRunStyle, text: &str) -> String {
    let preferred = if text.chars().any(|c| matches!(c as u32, 0xE000..=0xF8FF)) {
        style.symbol_typeface.as_deref()
    } else if text.chars().any(prep::is_east_asian) {
        style.east_asian_typeface.as_deref()
    } else if text
        .chars()
        .any(|c| matches!(c as u32, 0x0590..=0x10FF | 0x1780..=0x17FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF))
    {
        style.complex_script_typeface.as_deref()
    } else {
        style.latin_typeface.as_deref()
    };
    preferred
        .or(style.latin_typeface.as_deref())
        .or(style.east_asian_typeface.as_deref())
        .or(style.complex_script_typeface.as_deref())
        .or(style.symbol_typeface.as_deref())
        .unwrap_or("Arial")
        .to_string()
}

/// One font request: a requested face for one script's text, and the face
/// that draws it. rpptx sees it as the family `label`.
#[derive(Clone, Debug)]
struct Request {
    requested: String,
    script: Script,
    /// `None`: nothing here draws the script (no Korean font).
    drawn: Option<Drawn>,
    /// (bold, italic) variants the slides ask for.
    styles: BTreeSet<(bool, bool)>,
}

#[derive(Default)]
struct Requests {
    list: Vec<Request>,
    index: HashMap<(String, Script), usize>,
}

fn label(i: usize) -> String {
    format!("hanji-face-{i}")
}

impl Requests {
    /// The label for `requested` drawing `text`, noting the style.
    fn label(&mut self, requested: String, text: &str, bold: bool, italic: bool) -> String {
        let script = Script::of(text);
        let i = *self.index.entry((requested.clone(), script)).or_insert_with(|| {
            self.list.push(Request { requested, script, drawn: None, styles: BTreeSet::new() });
            self.list.len() - 1
        });
        self.list[i].styles.insert((bold, italic));
        label(i)
    }

    fn relabel_style(&mut self, style: &mut ResolvedRunStyle, text: &str) {
        let l = self.label(requested_typeface(style, text), text, style.bold, style.italic);
        style.latin_typeface = Some(l.clone());
        style.east_asian_typeface = Some(l.clone());
        style.complex_script_typeface = Some(l.clone());
        style.symbol_typeface = Some(l);
    }

    fn relabel_body(&mut self, body: &mut ResolvedTextBody) {
        for p in &mut body.paragraphs {
            let first = p.runs.iter().find_map(|r| match r {
                ResolvedTextRun::Text { style, .. } | ResolvedTextRun::Field { style, .. } => Some(style.clone()),
                ResolvedTextRun::Break => None,
            });
            let first = first.unwrap_or_else(|| p.end_style.clone());
            match &mut p.bullet {
                Some(ResolvedBullet::Character { character, font: Some(f), .. }) => {
                    *f = self.label(f.clone(), character, first.bold, first.italic);
                }
                Some(ResolvedBullet::AutoNumber { font: Some(f), .. }) => {
                    *f = self.label(f.clone(), "1", first.bold, first.italic);
                }
                _ => {}
            }
            for r in &mut p.runs {
                match r {
                    ResolvedTextRun::Text { text, style } | ResolvedTextRun::Field { text, style, .. } => {
                        self.relabel_style(style, text)
                    }
                    ResolvedTextRun::Break => {}
                }
            }
            self.relabel_style(&mut p.end_style, "");
        }
    }

    fn relabel_content(&mut self, content: &mut ResolvedContent) {
        match content {
            ResolvedContent::Text(body) => self.relabel_body(body),
            ResolvedContent::Table(t) => {
                for cell in t.rows.iter_mut().flat_map(|r| &mut r.cells) {
                    if let Some(body) = &mut cell.text {
                        self.relabel_body(body);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Glyph runs inside groups: rpptx lays out a group before the preview's
/// fonts apply, so their font ids are its deterministic manager's.
fn group_runs(elements: &mut [PositionedElement], f: &mut dyn FnMut(&mut FontId, &str)) {
    for e in elements {
        match e {
            PositionedElement::Text(r) => f(&mut r.font_id, &r.text),
            PositionedElement::MultilingualText(r) => f(&mut r.font_id, &r.logical_text),
            PositionedElement::Group(g) => group_runs(&mut g.children, f),
            PositionedElement::MarkedContent { children, .. } => group_runs(children, f),
            _ => {}
        }
    }
}

/// What the layout's face `FontId` stands for.
#[derive(Clone, Debug)]
struct FaceInfo {
    requested: String,
    script: Script,
    /// The drawn face's family, `None` when nothing drew the script.
    drawn: Option<String>,
    source: Option<Source>,
    metrics: Metrics,
}

impl FaceInfo {
    fn substituted(&self) -> bool {
        self.metrics != Metrics::Original
    }

    fn tooltip(&self) -> String {
        match &self.drawn {
            Some(d) => format!("{} → {d} (metrics: {})", self.requested, self.metrics.name()),
            None => format!("{}: no font draws this text", self.requested),
        }
    }
}

/// A rendered deck.
pub struct Preview {
    layout: LayoutResult,
    faces: HashMap<FontId, FaceInfo>,
    /// Characters drawn per page and face.
    chars: Vec<BTreeMap<FontId, BTreeSet<char>>>,
    pub fonts: FontsReport,
    /// What the reader should know: missing fonts, unreadable font directories, text rpptx laid out itself.
    pub warnings: Vec<String>,
}

/// `fonts` in the preview result (DESIGN.md §7.1).
#[derive(Clone, Debug, Default, Serialize)]
pub struct FontsReport {
    pub substituted: Vec<Substituted>,
    pub drawn_as_requested: Vec<AsRequested>,
    pub missing_glyphs: Vec<MissingGlyph>,
    /// How many distinct characters are missing (`missing_glyphs` lists at most 100).
    #[serde(skip_serializing_if = "is_zero")]
    pub missing_glyphs_total: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

#[derive(Clone, Debug, Serialize)]
pub struct Substituted {
    pub requested: String,
    pub script: Script,
    /// The face that draws it; `None` when no face draws this script.
    pub drawn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    pub metrics: Metrics,
    pub chars: usize,
    pub pages: Vec<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AsRequested {
    pub requested: String,
    pub script: Script,
    pub source: Source,
    pub chars: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct MissingGlyph {
    /// `U+AC00`.
    pub char: String,
    pub requested: String,
    pub pages: Vec<usize>,
}

const MISSING_LISTED: usize = 100;

/// "1, 3–5".
fn page_list(pages: &[usize]) -> String {
    let mut out: Vec<String> = vec![];
    let mut i = 0;
    while i < pages.len() {
        let mut j = i;
        while j + 1 < pages.len() && pages[j + 1] == pages[j] + 1 {
            j += 1;
        }
        out.push(if j > i + 1 {
            format!("{}–{}", pages[i], pages[j])
        } else if j == i + 1 {
            format!("{}, {}", pages[i], pages[j])
        } else {
            pages[i].to_string()
        });
        i = j + 1;
    }
    out.join(", ")
}

/// 1380 → "1,380".
fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

impl FontsReport {
    /// One line per substitution and missing glyph run (the §7.1 text form).
    pub fn lines(&self) -> Vec<String> {
        let mut out = vec![];
        for s in &self.substituted {
            let what = if s.script == Script::Latin { String::new() } else { format!(" ({} text)", s.script.name()) };
            let pages = format!("slide{} {}", if s.pages.len() == 1 { "" } else { "s" }, page_list(&s.pages));
            out.push(match &s.drawn {
                Some(d) => format!(
                    "{}{what} → {d}, {} chars, {pages} (metrics: {})",
                    s.requested,
                    thousands(s.chars),
                    s.metrics.name()
                ),
                None => format!("{}{what}: no font draws it, {} chars, {pages}", s.requested, thousands(s.chars)),
            });
        }
        if self.missing_glyphs_total > 0 {
            let shown: Vec<&str> = self.missing_glyphs.iter().take(8).map(|m| m.char.as_str()).collect();
            out.push(format!(
                "{} characters have no glyph in any font here ({}{})",
                thousands(self.missing_glyphs_total),
                shown.join(" "),
                if self.missing_glyphs_total > shown.len() { " …" } else { "" }
            ));
        }
        out
    }

    /// The one-line summary the CLI prints.
    pub fn summary(&self) -> String {
        if self.substituted.is_empty() && self.missing_glyphs_total == 0 {
            return format!("fonts: all {} as requested", self.drawn_as_requested.len());
        }
        // One entry per requested and drawn face, whatever the script.
        let mut merged: Vec<(&str, Option<&str>, usize)> = vec![];
        for s in &self.substituted {
            match merged.iter_mut().find(|m| m.0 == s.requested && m.1 == s.drawn.as_deref()) {
                Some(m) => m.2 += s.chars,
                None => merged.push((&s.requested, s.drawn.as_deref(), s.chars)),
            }
        }
        let subs: Vec<String> = merged
            .iter()
            .map(|(r, d, n)| format!("{r} → {} ({} chars)", d.unwrap_or("nothing"), thousands(*n)))
            .collect();
        let mut s = format!("fonts: {} substituted: {}", subs.len(), subs.join(", "));
        if !self.drawn_as_requested.is_empty() {
            s.push_str(&format!("; {} as requested", self.drawn_as_requested.len()));
        }
        if self.missing_glyphs_total > 0 {
            s.push_str(&format!("; {} characters missing", thousands(self.missing_glyphs_total)));
        }
        s
    }
}

/// The message for Korean text no face here draws.
pub const NO_KOREAN_FONT: &str = "no Korean font was found, so Hangul is drawn as empty boxes. Install Noto Sans CJK KR (Linux: the fonts-noto-cjk package; or from https://github.com/notofonts/noto-cjk), or pass --font-dir DIR (HANJI_FONT_DIR for the MCP server) with a Korean font such as 맑은 고딕.";

/// PowerPoint's single line, in em.
const LINE_EM: f64 = 1.2;

/// Renders a pptx package (the bytes export wrote).
pub fn render_pptx(package: &[u8], opts: &Options) -> Result<Preview, String> {
    let prepared = prep::prepare(package)?;
    let pres = rpptx::Presentation::from_bytes(&prepared).map_err(|e| format!("rpptx cannot open the deck: {e}"))?;
    let (mut input, stock) = pres.render_deterministic().map_err(|e| format!("rpptx cannot resolve the deck: {e}"))?;
    let fonts = Fonts::load(&opts.font_dirs, &input.fonts, opts.system_fonts);
    let mut warnings = fonts.warnings.clone();

    let mut req = Requests::default();
    for slide in &mut input.slides {
        for shape in &mut slide.shapes {
            req.relabel_content(&mut shape.content);
        }
    }
    // Group content was laid out with rpptx's deterministic fonts: its runs
    // ask for the face that drew them.
    let stock_fonts: HashMap<FontId, (String, bool, bool)> =
        stock.fonts.iter().map(|f| (f.id, (f.family.clone(), f.bold, f.italic))).collect();
    let mut grouped = 0;
    for shape in input.slides.iter_mut().flat_map(|s| &mut s.shapes) {
        if let ResolvedContent::Group(g) = &mut shape.content {
            group_runs(&mut g.children, &mut |id, text| {
                if let Some((family, bold, italic)) = stock_fonts.get(id) {
                    req.label(family.clone(), text, *bold, *italic);
                    grouped += 1;
                }
            });
        }
    }
    if grouped > 0 {
        warnings.push(format!(
            "{grouped} text runs in groups were laid out by rpptx with its built-in fonts (rpptx 0.12.1 lays out group content before the preview's fonts apply)"
        ));
    }

    let mut files = vec![];
    for (i, r) in req.list.iter_mut().enumerate() {
        r.drawn = fonts.resolve(&r.requested, r.script);
        // Text no face draws still takes up room: lay it out in the Latin face.
        let Some(face) = r.drawn.clone().or_else(|| fonts.resolve(&r.requested, Script::Latin)) else { continue };
        let mut seen = BTreeSet::new();
        for &(bold, italic) in &r.styles {
            let Some((data, index)) = fonts.face_data(&face, bold, italic) else { continue };
            let (b, it) =
                ttf_parser::Face::parse(&data, index).map(|f| (f.is_bold(), f.is_italic())).unwrap_or_default();
            if !seen.insert((b, it)) {
                continue;
            }
            let adj = sfnt::Adjust {
                family: label(i),
                bold: b,
                italic: it,
                line_em: Some(LINE_EM),
                ea_advance: face.ea_advance.filter(|_| r.drawn.is_some()),
            };
            match sfnt::build(&data, index, &adj) {
                Ok(data) => files.push(FontFile { family: label(i), data }),
                Err(e) => warnings.push(format!("the face {} for {} cannot be used: {e}", face.family, r.requested)),
            }
        }
    }
    if req.list.iter().any(|r| r.script == Script::Hangul && r.drawn.is_none()) {
        warnings.push(NO_KOREAN_FONT.to_string());
    }

    let mut fm = oxml_layout::FontManager::new_deterministic().map_err(|e| format!("fonts: {e}"))?;
    fm.load_additional_fonts(&files);
    for shape in input.slides.iter_mut().flat_map(|s| &mut s.shapes) {
        if let ResolvedContent::Group(g) = &mut shape.content {
            group_runs(&mut g.children, &mut |id, text| {
                let Some((family, bold, italic)) = stock_fonts.get(id) else { return };
                let l = req.index.get(&(family.clone(), Script::of(text))).map(|&i| label(i));
                if let Some(new) = l.and_then(|l| fm.resolve_font(Some(&l), *bold, *italic).ok()) {
                    *id = new;
                }
            });
        }
    }
    let layout = rpptx_render::layout_presentation_with_font_manager(&input, fm)
        .map_err(|e| format!("rpptx cannot lay out the deck: {e}"))?;

    let faces: HashMap<FontId, FaceInfo> = layout
        .fonts
        .iter()
        .map(|f| {
            let info = f
                .family
                .strip_prefix("hanji-face-")
                .and_then(|n| n.parse::<usize>().ok())
                .and_then(|i| req.list.get(i))
                .map(|r| FaceInfo {
                    requested: r.requested.clone(),
                    script: r.script,
                    drawn: r.drawn.as_ref().map(|d| d.family.clone()),
                    source: r.drawn.as_ref().map(|d| d.source),
                    metrics: r.drawn.as_ref().map_or(Metrics::Substitute, |d| d.metrics.clone()),
                })
                // A face rpptx chose itself (a chart's, or a fallback for characters the requested face lacks).
                .unwrap_or_else(|| FaceInfo {
                    requested: f.family.clone(),
                    script: Script::Latin,
                    drawn: Some(f.family.clone()),
                    source: Some(Source::Bundled),
                    metrics: Metrics::Original,
                });
            (f.id, info)
        })
        .collect();
    let mut p = Preview { layout, faces, chars: vec![], fonts: FontsReport::default(), warnings };
    p.count();
    Ok(p)
}

/// The glyph runs of `elements` (groups included): font, text.
fn runs<'a>(elements: &'a [PositionedElement], out: &mut Vec<(FontId, &'a str)>) {
    for e in elements {
        match e {
            PositionedElement::Text(r) => out.push((r.font_id, &r.text)),
            PositionedElement::MultilingualText(r) => out.push((r.font_id, &r.logical_text)),
            PositionedElement::Group(g) => runs(&g.children, out),
            PositionedElement::MarkedContent { children, .. } => runs(children, out),
            _ => {}
        }
    }
}

impl Preview {
    pub fn slide_count(&self) -> usize {
        self.layout.pages.len()
    }

    /// Characters per page and face, and the fonts report.
    fn count(&mut self) {
        type Key = (String, Script, Option<String>, Option<Source>, Metrics);
        let mut subs: BTreeMap<Key, (usize, BTreeSet<usize>)> = BTreeMap::new();
        let mut asked: BTreeMap<(String, Script, Source), usize> = BTreeMap::new();
        let mut missing: BTreeMap<char, (String, BTreeSet<usize>)> = BTreeMap::new();
        let data: HashMap<FontId, (Arc<[u8]>, u32)> =
            self.layout.fonts.iter().map(|f| (f.id, (f.data.clone(), f.face_index))).collect();
        for (k, page) in self.layout.pages.iter().enumerate() {
            let mut list = vec![];
            runs(&page.elements, &mut list);
            let mut per_font: BTreeMap<FontId, BTreeSet<char>> = BTreeMap::new();
            for (id, text) in list {
                let Some(info) = self.faces.get(&id) else { continue };
                // Spaces go into the subsets (a renderer without them draws `.notdef`), not into the counts.
                per_font.entry(id).or_default().extend(text.chars().filter(|c| !c.is_control()));
                let drawn: Vec<char> = text.chars().filter(|c| !c.is_whitespace() && !c.is_control()).collect();
                if drawn.is_empty() {
                    continue;
                }
                let face = data.get(&id).and_then(|(d, i)| ttf_parser::Face::parse(d, *i).ok());
                for &c in &drawn {
                    if face.as_ref().is_none_or(|f| f.glyph_index(c).is_none()) {
                        missing.entry(c).or_insert_with(|| (info.requested.clone(), BTreeSet::new())).1.insert(k + 1);
                    }
                }
                if info.substituted() || info.drawn.is_none() {
                    let key =
                        (info.requested.clone(), info.script, info.drawn.clone(), info.source, info.metrics.clone());
                    let e = subs.entry(key).or_default();
                    e.0 += drawn.len();
                    e.1.insert(k + 1);
                } else if let Some(src) = info.source {
                    *asked.entry((info.requested.clone(), info.script, src)).or_default() += drawn.len();
                }
            }
            self.chars.push(per_font);
        }
        let mut substituted: Vec<Substituted> = subs
            .into_iter()
            .map(|((requested, script, drawn, source, metrics), (chars, pages))| Substituted {
                requested,
                script,
                drawn,
                source,
                metrics,
                chars,
                pages: pages.into_iter().collect(),
            })
            .collect();
        substituted.sort_by_key(|s| std::cmp::Reverse(s.chars));
        self.fonts = FontsReport {
            substituted,
            drawn_as_requested: asked
                .into_iter()
                .map(|((requested, script, source), chars)| AsRequested { requested, script, source, chars })
                .collect(),
            missing_glyphs_total: missing.len(),
            missing_glyphs: missing
                .into_iter()
                .take(MISSING_LISTED)
                .map(|(c, (requested, pages))| MissingGlyph {
                    char: format!("U+{:04X}", c as u32),
                    requested,
                    pages: pages.into_iter().collect(),
                })
                .collect(),
        };
    }

    /// Each face's subset for `chars`, as (font, subset bytes); a face that
    /// cannot be subset is left out (its text falls back in the viewer).
    fn subsets(&self, chars: &BTreeMap<FontId, BTreeSet<char>>) -> Vec<(FontId, Vec<u8>)> {
        chars
            .iter()
            .filter_map(|(id, cs)| {
                let f = self.layout.fonts.iter().find(|f| f.id == *id)?;
                subset::subset(&f.data, f.face_index, cs).ok().map(|d| (*id, d))
            })
            .collect()
    }

    fn font_css(subsets: &[(FontId, Vec<u8>)]) -> String {
        subsets
            .iter()
            .map(|(id, d)| {
                let (b, i) = ttf_parser::Face::parse(d, 0).map(|f| (f.is_bold(), f.is_italic())).unwrap_or_default();
                subset::font_face(id.0, d, b, i)
            })
            .collect()
    }

    fn page_svg(&self, k: usize, css: String, marks: bool) -> String {
        let hooks = PageHooks { preview: self, css, marks };
        svg::render_page(&self.layout, k, &hooks).map(|r| r.svg).unwrap_or_default()
    }

    /// Slide `k` (0-based) as a standalone SVG with its fonts subset to the
    /// slide's characters and embedded.
    pub fn slide_svg(&self, k: usize) -> String {
        let subsets = self.subsets(&self.chars[k]);
        self.page_svg(k, Self::font_css(&subsets), false)
    }

    /// Slide `k` (0-based) as PNG at `dpi`, drawn with the same subset faces.
    pub fn slide_png(&self, k: usize, dpi: f64) -> Result<Vec<u8>, String> {
        use resvg::{tiny_skia, usvg};
        let subsets = self.subsets(&self.chars[k]);
        let svg = self.page_svg(k, String::new(), false);
        let mut options = usvg::Options::default();
        let mut exact = HashMap::new();
        for (id, data) in subsets {
            let ids = options.fontdb_mut().load_font_source(usvg::fontdb::Source::Binary(Arc::new(data)));
            if let Some(face) = ids.first() {
                exact.insert(format!("hanji-font-{}", id.0), *face);
            }
        }
        options.font_resolver = usvg::FontResolver {
            select_font: Box::new(move |font, _| {
                font.families().iter().find_map(|f| exact.get(f.to_string().trim_matches('"')).copied())
            }),
            select_fallback: Box::new(|_, _, _| None),
        };
        let tree = usvg::Tree::from_str(&svg, &options).map_err(|e| format!("usvg: {e}"))?;
        let page = &self.layout.pages[k];
        let scale = dpi / 72.0;
        let (w, h) = ((page.width * scale).round() as u32, (page.height * scale).round() as u32);
        let mut pm = tiny_skia::Pixmap::new(w.max(1), h.max(1)).ok_or("the slide is too large to draw")?;
        pm.fill(tiny_skia::Color::WHITE);
        let (sx, sy) = (w as f32 / tree.size().width(), h as f32 / tree.size().height());
        resvg::render(&tree, tiny_skia::Transform::from_scale(sx, sy), &mut pm.as_mut());
        pm.encode_png().map_err(|e| format!("png: {e}"))
    }

    /// The viewer: one self-contained HTML file, slides stacked, each face
    /// subset once for the whole deck, substituted text marked.
    pub fn html(&self, title: &str) -> String {
        let mut all: BTreeMap<FontId, BTreeSet<char>> = BTreeMap::new();
        for page in &self.chars {
            for (id, cs) in page {
                all.entry(*id).or_default().extend(cs);
            }
        }
        let css = Self::font_css(&self.subsets(&all));
        let esc = hanji_package::xml::escape_text;
        let mut banner = String::new();
        let lines = self.fonts.lines();
        if lines.is_empty() && self.warnings.is_empty() {
            banner.push_str("<p>Every font is drawn as requested.</p>");
        } else {
            banner.push_str("<ul>");
            for l in self.warnings.iter().chain(&lines) {
                banner.push_str(&format!("<li>{}</li>", esc(l)));
            }
            banner.push_str("</ul>");
        }
        let mut slides = String::new();
        for k in 0..self.slide_count() {
            let svg = self.page_svg(k, String::new(), true);
            // Sized by the page's CSS, not in points.
            let svg = match (svg.find(" width=\""), svg.find(" viewBox=")) {
                (Some(a), Some(b)) if a < b => format!("{} class=\"slide-svg\"{}", &svg[..a], &svg[b..]),
                _ => svg,
            };
            slides.push_str(&format!(
                "<section class=\"slide\" id=\"slide-{n}\"><a class=\"num\" href=\"#slide-{n}\">{n}</a>{svg}</section>\n",
                n = k + 1
            ));
        }
        format!(
            r#"<!doctype html>
<html{lang}>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
{css}
:root {{ color-scheme: light; }}
body {{ margin: 0; background: #E8E8EC; font: 14px/1.5 system-ui, sans-serif; color: #1A1A1A; }}
header {{ position: sticky; top: 0; z-index: 1; background: #FFFFFF; border-bottom: 1px solid #C8C8D0; padding: 8px 16px; }}
header h1 {{ font-size: 15px; margin: 0 0 4px; }}
header ul {{ margin: 0; padding-left: 20px; }}
header label {{ display: inline-block; margin-top: 4px; }}
main {{ max-width: 1100px; margin: 0 auto; padding: 16px; }}
.slide {{ position: relative; margin: 0 0 20px; background: #FFFFFF; box-shadow: 0 1px 4px rgba(0, 0, 0, .25); }}
.slide-svg {{ display: block; width: 100%; height: auto; }}
.num {{ position: absolute; left: -2px; top: -2px; transform: translateX(-100%); font-size: 12px; color: #555; text-decoration: none; padding-right: 6px; }}
.hide-marks .hanji-mark {{ display: none; }}
</style>
</head>
<body>
<header>
<h1>{title}</h1>
{banner}
<label><input type="checkbox" checked onchange="document.body.classList.toggle('hide-marks', !this.checked)"> Mark substituted text (dotted underline; hover for the fonts)</label>
</header>
<main>
{slides}</main>
</body>
</html>
"#,
            title = esc(title),
            lang = if self.faces.values().any(|f| f.script == Script::Hangul) { " lang=\"ko\"" } else { "" },
        )
    }
}

struct PageHooks<'a> {
    preview: &'a Preview,
    css: String,
    marks: bool,
}

impl svg::Hooks for PageHooks<'_> {
    fn font_css(&self, _used: &[FontId]) -> String {
        self.css.clone()
    }

    fn text_attributes(&self, font: FontId) -> String {
        let attr = hanji_package::xml::escape_attr;
        match self.preview.faces.get(&font) {
            Some(f) => format!(
                " data-font-requested=\"{}\" data-font-drawn=\"{}\"",
                attr(&f.requested),
                attr(f.drawn.as_deref().unwrap_or(""))
            ),
            None => String::new(),
        }
    }

    fn mark(&self, font: FontId) -> Option<String> {
        let f = self.preview.faces.get(&font).filter(|f| self.marks && (f.substituted() || f.drawn.is_none()))?;
        Some(f.tooltip())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_lists_and_counts_read_well() {
        assert_eq!(page_list(&[1, 2, 3, 5, 7, 8]), "1–3, 5, 7, 8");
        assert_eq!(page_list(&[4]), "4");
        assert_eq!(thousands(1380), "1,380");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000_000), "1,000,000");
    }

    #[test]
    fn the_requested_typeface_follows_the_text() {
        let style = ResolvedRunStyle {
            latin_typeface: Some("Calibri".into()),
            east_asian_typeface: Some("맑은 고딕".into()),
            ..Default::default()
        };
        assert_eq!(requested_typeface(&style, "Q3"), "Calibri");
        assert_eq!(requested_typeface(&style, "매출"), "맑은 고딕");
        assert_eq!(requested_typeface(&ResolvedRunStyle::default(), "x"), "Arial");
        let latin_only = ResolvedRunStyle { latin_typeface: Some("Calibri".into()), ..Default::default() };
        assert_eq!(requested_typeface(&latin_only, "매출"), "Calibri");
    }
}
