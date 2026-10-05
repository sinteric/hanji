//! Read-only worksheet windows from package bytes. This is a grid projection,
//! not Excel print layout or a calculation API. No macros, links or formulas execute.
mod styles;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use hanji_core::cells::{col_letters, CellRange, CellRef, MAX_COL, MAX_ROW};
use hanji_package::{opc, package, xml};
use hanji_xlsx::book::{Book, SheetKind, Shell};
use hanji_xlsx::store::{Cell, Row};
use hanji_xlsx::value::CellValue;
use oxml_layout::{
    Color, FontId, GlyphRun, GroupElement, LayoutResult, PageFrame, Path, Point, PositionedElement, Rect, Transform,
};
use serde::Serialize;

use crate::{
    fonts::{Fonts, Script},
    Diagnostic, FaceInfo, FontOptions, FontResolver, FontsReport, PageFormat, PageInfo, Preview, RenderedPage,
};
use styles::Style;

/// Configurable workload budgets, independent of Excel's address limits.
/// Exceeding one is an explicit error; no cells/results are silently truncated.
#[derive(Clone, Debug)]
pub struct XlsxOptions {
    pub max_unpacked_bytes: u64,
    pub max_window_rows: u32,
    pub max_window_columns: u32,
    pub max_window_cells: u64,
    pub max_window_text_bytes: usize,
    pub max_window_merges: usize,
    pub max_png_pixels: u64,
    pub max_font_family_bytes: usize,
    pub max_number_format_bytes: usize,
}
impl Default for XlsxOptions {
    fn default() -> Self {
        Self {
            max_unpacked_bytes: 64 << 20,
            max_window_rows: 512,
            max_window_columns: 128,
            max_window_cells: 32_768,
            max_window_text_bytes: 2 << 20,
            max_window_merges: 4096,
            max_png_pixels: 16_777_216,
            max_font_family_bytes: 1024,
            max_number_format_bytes: 4096,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct SheetInfo {
    pub index: usize,
    pub name: String,
    pub state: String,
    pub is_worksheet: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FormulaResult {
    NotFormula,
    CachedUnverified,
    CachedPossiblyStale,
    Missing,
}
#[derive(Clone, Debug, Serialize)]
pub struct CellInfo {
    pub address: String,
    pub display: String,
    /// Stored text only; shared-formula follower expressions are not expanded.
    pub formula: Option<String>,
    pub formula_result: FormulaResult,
}

/// Holds sparse package rows, shared strings and styles. A worksheet is indexed
/// on first access; only requested rows become visual layout elements.
pub struct Workbook {
    book: Book,
    styles: Vec<Style>,
    normal_font: Style,
    options: XlsxOptions,
    possibly_stale: bool,
    pub sheets: Vec<SheetInfo>,
    pub diagnostics: Vec<Diagnostic>,
}
/// A single window is one coarse render job, using the same embedded-font and
/// SVG/PNG pipeline as presentation pages. Retain/drop windows independently.
pub struct Window {
    preview: Preview,
    max_png_pixels: u64,
    pub sheet: SheetInfo,
    pub range: String,
    pub cells: Vec<CellInfo>,
}
fn diagnostic(path: impl Into<String>, message: impl Into<String>) -> Diagnostic {
    Diagnostic { path: path.into(), message: message.into() }
}
fn yes(s: Option<String>) -> bool {
    s.is_some_and(|s| s == "1" || s == "true")
}
fn attr(attrs: &[(String, String)], key: &str) -> Option<String> {
    attrs.iter().find(|(k, _)| k == key).map(|(_, v)| xml::unescape(v))
}
fn size(s: Option<String>, default: f64, max: f64) -> Result<f64, String> {
    let n = match s {
        Some(s) => s.parse::<f64>().map_err(|_| "invalid row/column size")?,
        None => default,
    };
    if !n.is_finite() || n < 0.0 || n > max {
        return Err("row/column size outside supported numeric range".into());
    }
    Ok(n)
}

pub fn open_xlsx(bytes: &[u8], options: XlsxOptions) -> Result<Workbook, String> {
    if options.max_unpacked_bytes == 0
        || options.max_unpacked_bytes > package::MAX_UNPACKED
        || options.max_window_rows == 0
        || options.max_window_columns == 0
        || options.max_window_cells == 0
        || options.max_window_text_bytes == 0
        || options.max_window_merges == 0
        || options.max_png_pixels == 0
        || options.max_font_family_bytes == 0
        || options.max_number_format_bytes == 0
    {
        return Err(
            "XLSX preview budgets must be positive; package budget cannot exceed the package reader's limit".into()
        );
    }
    let parts = package::read_limited(bytes, options.max_unpacked_bytes)?;
    let mut diagnostics = vec![];
    if parts.iter().any(|p| p.name.to_ascii_lowercase().ends_with("vbaproject.bin")) {
        diagnostics.push(diagnostic("workbook.macros", "macro content is ignored and never executed"));
    }
    if parts.iter().filter(|p| p.name.ends_with(".rels")).any(|p| opc::parse_rels(&p.data).iter().any(|r| r.external)) {
        diagnostics.push(diagnostic(
            "workbook.externalLinks",
            "external relationships are not fetched, activated or refreshed",
        ));
    }
    let book = Book::load(parts, vec![], Shell::default())?;
    if book.styles.fmts.values().any(|s| s.len() > options.max_number_format_bytes) {
        return Err("number format exceeds the configured metadata budget".into());
    }
    let (styles, normal_font) = styles::parse(
        book.styles.part.as_deref().and_then(|p| package::get(&book.parts, p)),
        options.max_font_family_bytes,
    )?;
    let possibly_stale = book.wb.root.elements().find(|e| e.local() == "calcPr").is_some_and(|c| {
        yes(c.get("fullCalcOnLoad"))
            || yes(c.get("forceFullCalc"))
            || c.get("calcMode").as_deref() == Some("manual")
            || c.get("calcCompleted").is_some_and(|v| v == "0" || v == "false")
    });
    let sheets = book
        .sheets
        .iter()
        .enumerate()
        .map(|(index, s)| SheetInfo {
            index,
            name: s.name.clone(),
            state: s.state.clone(),
            is_worksheet: s.kind == SheetKind::Work,
        })
        .collect();
    Ok(Workbook { book, styles, normal_font, options, possibly_stale, sheets, diagnostics })
}

impl Workbook {
    /// Native host-font adapter. Selection/layout stay in this library; the
    /// byte-only APIs remain available without filesystem or host discovery.
    #[cfg(all(feature = "host-fonts", not(target_family = "wasm")))]
    pub fn render_window(&mut self, sheet: usize, range: &str, opts: &crate::Options) -> Result<Window, String> {
        let window = {
            let fonts = Fonts::load(&opts.font_dirs, &[], opts.system_fonts);
            self.layout_window(sheet, range, &fonts)?
        };
        finish_window(window)
    }

    /// Range uses Excel A1 addresses. Hidden rows/columns collapse; hidden sheets
    /// are listed with their state and can be accessed only by explicit index.
    pub fn render_window_with_fonts(
        &mut self,
        sheet: usize,
        range: &str,
        fonts: &FontOptions,
    ) -> Result<Window, String> {
        let window = {
            let resolver = Fonts::from_bytes(&fonts.fonts, &[], fonts.aliases.clone())?;
            self.layout_window(sheet, range, &resolver)?
        };
        finish_window(window)
    }

    pub fn render_window_with_resolver(
        &mut self,
        sheet: usize,
        range: &str,
        fonts: &dyn FontResolver,
    ) -> Result<Window, String> {
        finish_window(self.layout_window(sheet, range, fonts)?)
    }

    fn layout_window(&mut self, sheet: usize, range: &str, fonts: &dyn FontResolver) -> Result<Window, String> {
        let range = CellRange::parse(range).ok_or("invalid worksheet window; expected an A1 range")?;
        if range.rows() > self.options.max_window_rows
            || range.cols() > self.options.max_window_columns
            || u64::from(range.rows()) * u64::from(range.cols()) > self.options.max_window_cells
        {
            return Err(
                "worksheet window exceeds the configured row/column/cell budget; request a smaller range".into()
            );
        }
        let info = self.sheets.get(sheet).ok_or("worksheet index is out of range")?.clone();
        if !info.is_worksheet {
            return Err("chart/macro/dialog sheets do not have a worksheet grid preview".into());
        }
        self.book.load_store(sheet)?;
        let store = self.book.store(sheet);
        let prefix = format!("sheets[{sheet}]");
        let mut diagnostics = self.diagnostics.clone();
        diagnostics.push(diagnostic(format!("{prefix}.grid"), "worksheet window only: column widths use a 7-pixel maximum-digit approximation; text wraps by character, clips to cells, and stored row heights are not auto-fitted; rich-text runs use the cell font; print pagination is not applied"));
        let mut merges = vec![];
        if let Some(list) = store.child("mergeCells") {
            for e in list.elements().filter(|e| e.local() == "mergeCell") {
                let m = e.get("ref").and_then(|s| CellRange::parse(&s)).ok_or("invalid merged-cell range")?;
                if !m.intersects(&range) {
                    continue;
                }
                if merges.len() >= self.options.max_window_merges {
                    return Err("window exceeds the configured merged-range budget".into());
                }
                if merges.iter().any(|o: &CellRange| o.intersects(&m)) {
                    return Err("overlapping merged-cell ranges are not supported".into());
                }
                if m.intersects(&range) && (!range.contains(m.first) || !range.contains(m.last)) {
                    return Err(format!(
                        "window cuts merged range {m}; request a window containing that complete range"
                    ));
                }
                merges.push(m);
            }
        }
        for feature in [
            "drawing",
            "legacyDrawing",
            "conditionalFormatting",
            "tableParts",
            "dataValidations",
            "hyperlinks",
            "autoFilter",
            "sheetProtection",
            "extLst",
            "pageSetup",
            "pageMargins",
            "headerFooter",
            "rowBreaks",
            "colBreaks",
            "sheetViews",
        ] {
            if store.child(feature).is_some() {
                diagnostics.push(diagnostic(format!("{prefix}.{feature}"), format!("{feature} semantics are not applied in the worksheet grid (charts/images, rules, table styles, filters, protection, links, freeze panes and print settings are separate work)")));
            }
        }
        let mut rows = BTreeMap::<u32, Row>::new();
        for r in range.first.row..=range.last.row {
            if let Some(row) = store.try_row(r).map_err(|e| format!("{prefix}.rows[{r}]: {e}"))? {
                rows.insert(r, row);
            }
        }
        let format = store.child("sheetFormatPr");
        let default_height = size(format.and_then(|f| f.get("defaultRowHeight")), 15.0, 409.0)?;
        let default_width = size(format.and_then(|f| f.get("defaultColWidth")), 8.43, 255.0)?;
        let mut widths = vec![(default_width * 7.0 + 5.0) * 0.75; range.cols() as usize];
        let mut col_styles = vec![0usize; widths.len()];
        if let Some(cols) = store.child("cols") {
            for c in cols.elements().filter(|c| c.local() == "col") {
                let min = c.get("min").and_then(|s| s.parse::<u32>().ok()).ok_or("invalid column min")?;
                let max = c.get("max").and_then(|s| s.parse::<u32>().ok()).ok_or("invalid column max")?;
                if min == 0 || min > max || max > MAX_COL {
                    return Err("invalid column span".into());
                }
                let width = size(c.get("width"), default_width, 255.0)?;
                let width = if yes(c.get("hidden")) || width == 0.0 { 0.0 } else { (width * 7.0 + 5.0) * 0.75 };
                for col in (min - 1).max(range.first.col)..=(max - 1).min(range.last.col) {
                    widths[(col - range.first.col) as usize] = width;
                    col_styles[(col - range.first.col) as usize] =
                        c.get("style").and_then(|s| s.parse().ok()).unwrap_or(0);
                }
            }
        }
        let default_hidden = yes(format.and_then(|f| f.get("zeroHeight")));
        let mut heights = vec![if default_hidden { 0.0 } else { default_height }; range.rows() as usize];
        for (&r, row) in &rows {
            if r == 0 || r > MAX_ROW {
                return Err("invalid row address".into());
            }
            heights[(r - range.first.row) as usize] =
                if attr(&row.attrs, "hidden").map_or(default_hidden, |v| v == "1" || v == "true") {
                    0.0
                } else {
                    size(attr(&row.attrs, "ht"), default_height, 409.0)?
                };
        }
        let offsets = |sizes: &[f64], start: f64| {
            let mut out = vec![start];
            for n in sizes {
                out.push(out.last().unwrap() + n);
            }
            out
        };
        let xs = offsets(&widths, 36.0);
        let ys = offsets(&heights, 18.0);
        let mut builder = GridBuilder::new(fonts, &self.normal_font, self.options.max_window_text_bytes, diagnostics);
        for (ci, &w) in widths.iter().enumerate().filter(|(_, w)| **w > 0.0) {
            let rect = Rect { x: xs[ci], y: 0.0, width: w, height: 18.0 };
            builder.cell(rect, &col_letters(range.first.col + ci as u32), &Style::default(), false, false, "header")?;
        }
        let mut cells = vec![];
        let mut used_styles = BTreeSet::new();
        for (ri, &h) in heights.iter().enumerate().filter(|(_, h)| **h > 0.0) {
            let r = range.first.row + ri as u32;
            builder.cell(
                Rect { x: 0.0, y: ys[ri], width: 36.0, height: h },
                &r.to_string(),
                &Style::default(),
                true,
                false,
                "header",
            )?;
            let row = rows.get(&r);
            let values: BTreeMap<u32, &Cell> = row.into_iter().flat_map(|r| &r.cells).map(|c| (c.col, c)).collect();
            if row.is_some_and(|row| {
                row.r != r
                    || values.len() != row.cells.len()
                    || row.cells.iter().any(|c| {
                        c.col >= MAX_COL
                            || c.get("r").and_then(CellRef::parse).is_none_or(|a| a.row != r || a.col != c.col)
                    })
            }) {
                return Err(format!("{prefix}.rows[{r}]: duplicate or invalid cell column"));
            }
            for (ci, _) in widths.iter().enumerate().filter(|(_, w)| **w > 0.0) {
                let col = range.first.col + ci as u32;
                let address = CellRef::new(col, r);
                let merged = merges.iter().find(|m| m.contains(address));
                if merged.is_some_and(|m| m.first != address) {
                    continue;
                }
                let (last_col, last_row) = merged.map_or((col, r), |m| (m.last.col, m.last.row));
                let rect = Rect {
                    x: xs[ci],
                    y: ys[ri],
                    width: xs[(last_col - range.first.col + 1) as usize] - xs[ci],
                    height: ys[(last_row - range.first.row + 1) as usize] - ys[ri],
                };
                let cell = values.get(&col).copied();
                let style_index = cell
                    .and_then(|c| c.get("s"))
                    .and_then(|s| s.parse().ok())
                    .or_else(|| {
                        row.filter(|r| yes(attr(&r.attrs, "customFormat")))
                            .and_then(|r| attr(&r.attrs, "s"))
                            .and_then(|s| s.parse().ok())
                    })
                    .unwrap_or(col_styles[ci]);
                let style = self
                    .styles
                    .get(style_index)
                    .ok_or_else(|| format!("{prefix}.cells[{address}]: invalid cell style {style_index}"))?;
                if used_styles.insert(style_index) {
                    for loss in &style.losses {
                        builder.diagnostics.push(diagnostic(format!("{prefix}.styles[{style_index}]"), loss));
                    }
                }
                let path = format!("{prefix}.cells[{address}]");
                if let Some(c) = cell {
                    if c.ty() == "s"
                        && c.v
                            .as_deref()
                            .and_then(|s| s.parse::<usize>().ok())
                            .and_then(|i| self.book.sst.as_ref()?.get(i))
                            .is_none()
                    {
                        return Err(format!("{path}: invalid shared-string reference"));
                    }
                    if c.ty() == "b"
                        && c.v.as_deref().is_some_and(|v| !matches!(v.trim(), "0" | "1" | "true" | "false"))
                    {
                        return Err(format!("{path}: invalid boolean value"));
                    }
                    if c.ty() == "n"
                        && c.v.as_deref().and_then(|s| s.parse::<f64>().ok()).is_some_and(|v| !v.is_finite())
                    {
                        return Err(format!("{path}: nonfinite numeric value"));
                    }
                    if !matches!(c.ty(), "n" | "s" | "str" | "inlineStr" | "b" | "e" | "d") {
                        return Err(format!("{path}: unsupported cell value type {:?}", c.ty()));
                    }
                }
                // Use the same effective style as the visual projection: a cell
                // without `s` can inherit its number format from the row/column.
                let mut display = cell
                    .map(|c| {
                        let format = self.book.styles.format_of(style_index as u32);
                        hanji_xlsx::numfmt::display(&self.book.value(c), &format, self.book.date1904)
                    })
                    .unwrap_or_default();
                let formula = cell.and_then(Cell::formula);
                let formula_result = if let Some(c) = cell.filter(|c| c.f.is_some()) {
                    if c.lacks_cached_value()
                        || (c.ty() != "str" && c.v.as_deref().is_some_and(|v| v.trim().is_empty()))
                    {
                        display = "#UNEVALUATED".into();
                        builder
                            .diagnostics
                            .push(diagnostic(format!("{path}.formula"), crate::quality::FORMULA_MISSING));
                        FormulaResult::Missing
                    } else {
                        builder.diagnostics.push(diagnostic(format!("{path}.formula"), if self.possibly_stale {
                            "stored formula result shown; workbook requests recalculation or uses manual calculation, so this cache may be stale; no evaluation was performed"
                        } else { "stored formula result shown; cache freshness is unverified; no evaluation was performed" }));
                        if self.possibly_stale {
                            FormulaResult::CachedPossiblyStale
                        } else {
                            FormulaResult::CachedUnverified
                        }
                    }
                } else {
                    FormulaResult::NotFormula
                };
                let numeric = cell.is_some_and(|c| c.ty() == "d" || matches!(self.book.value(c), CellValue::Number(_)));
                builder.cell(rect, &display, style, numeric, formula_result == FormulaResult::Missing, &path)?;
                if cell.is_some() {
                    cells.push(CellInfo { address: address.to_string(), display, formula, formula_result });
                }
            }
        }
        let preview = builder.finish(*xs.last().unwrap(), *ys.last().unwrap())?;
        Ok(Window {
            preview,
            max_png_pixels: self.options.max_png_pixels,
            sheet: info,
            range: range.to_string(),
            cells,
        })
    }
}

impl Window {
    pub fn page_info(&self) -> PageInfo {
        self.preview.page_info(0).unwrap()
    }
    pub fn fonts(&self) -> &FontsReport {
        &self.preview.fonts
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.preview.diagnostics
    }
    pub fn warnings(&self) -> &[String] {
        &self.preview.warnings
    }
    pub fn render(&self, format: PageFormat) -> Result<RenderedPage, String> {
        if let PageFormat::Png { dpi } = format {
            let p = self.page_info();
            let (w, h) = crate::png_dimensions(p.width, p.height, dpi)?;
            if u64::from(w) * u64::from(h) > self.max_png_pixels {
                return Err(
                    "worksheet PNG exceeds the configured pixel budget; lower DPI or request a smaller window".into()
                );
            }
        }
        let mut output = self.preview.render_page(0, format)?;
        if let crate::PageData::Svg(svg) = &mut output.data {
            *svg = self.accessible_svg(svg);
        }
        output.diagnostics = self.preview.diagnostics.clone();
        Ok(output)
    }
    /// A self-contained static window. No scripts, external links or event handlers.
    pub fn html(&self, title: &str) -> String {
        let esc = crate::svg::escape_xml_text;
        let warnings: String = self.preview.warnings.iter().map(|w| format!("<li>{}</li>", esc(w))).collect();
        let cells: String = self
            .cells
            .iter()
            .map(|c| {
                format!(
                    "<tr><th scope=\"row\">{}</th><td>{}</td><td>{}</td></tr>",
                    esc(&c.address),
                    esc(&c.display),
                    formula_description(&c.formula_result)
                )
            })
            .collect();
        format!("<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body><h1>{}</h1><p>{}: {} — worksheet window, cached formula results only</p>{}<ul>{warnings}</ul>{}<details><summary>Complete cell values</summary><table><caption>Complete displayed values and formula cache state</caption><thead><tr><th>Cell</th><th>Display</th><th>Formula cache</th></tr></thead><tbody>{cells}</tbody></table></details></body></html>", esc(title), esc(title), esc(&self.sheet.name), esc(&self.range), self.quality().html_notice(), self.accessible_svg(&self.preview.slide_svg(0)))
    }

    fn accessible_svg(&self, svg: &str) -> String {
        let description: String = self
            .cells
            .iter()
            .map(|c| format!("{}: {}; {}. ", c.address, c.display, formula_description(&c.formula_result)))
            .collect();
        let heading = format!("{}: {}", self.sheet.name, self.range);
        let prefix = svg.find('>').expect("own SVG has a root start tag");
        format!("{} role=\"img\" aria-labelledby=\"hanji-xlsx-title\" aria-describedby=\"hanji-xlsx-description\"><title id=\"hanji-xlsx-title\">{}</title><desc id=\"hanji-xlsx-description\">{}Diagnostic coverage is partial; native Excel fidelity is unverified.</desc>{}", &svg[..prefix], crate::svg::escape_xml_text(&heading), crate::svg::escape_xml_text(&description), &svg[prefix+1..])
    }

    pub fn quality(&self) -> crate::quality::QualityReport {
        crate::quality::QualityReport::inspect(
            crate::quality::PreviewSource::Xlsx,
            self.diagnostics(),
            self.fonts(),
            self.warnings(),
        )
    }
}

fn formula_description(result: &FormulaResult) -> &'static str {
    match result {
        FormulaResult::NotFormula => "not a formula",
        FormulaResult::CachedUnverified => "stored formula cache; freshness unverified",
        FormulaResult::CachedPossiblyStale => "stored formula cache; possibly stale",
        FormulaResult::Missing => "missing formula cache; unevaluated",
    }
}

fn finish_window(mut window: Window) -> Result<Window, String> {
    // Drop worksheet layout/font-selection temporaries before subset preflight.
    window.preview.count();
    window.preview.prepare_output()?;
    Ok(window)
}

struct GridBuilder<'a> {
    resolver: &'a dyn FontResolver,
    normal_font: &'a Style,
    normal_space: Option<f64>,
    elements: Vec<PositionedElement>,
    fonts: Vec<oxml_layout::FontData>,
    faces: HashMap<FontId, FaceInfo>,
    keys: BTreeMap<(String, Script, bool, bool), Vec<FontId>>,
    diagnostics: Vec<Diagnostic>,
    remaining_text: usize,
}
impl<'a> GridBuilder<'a> {
    fn new(
        resolver: &'a dyn FontResolver,
        normal_font: &'a Style,
        remaining_text: usize,
        diagnostics: Vec<Diagnostic>,
    ) -> Self {
        Self {
            resolver,
            normal_font,
            normal_space: None,
            elements: vec![],
            fonts: vec![],
            faces: HashMap::new(),
            keys: BTreeMap::new(),
            diagnostics,
            remaining_text,
        }
    }
    fn indentation(&mut self, levels: u32) -> Result<f64, String> {
        if levels == 0 {
            return Ok(0.0);
        }
        let space = if let Some(space) = self.normal_space {
            space
        } else {
            let s = self.normal_font;
            let resolved = self
                .resolver
                .resolve_font_for_text(&s.family, Script::Latin, s.bold, s.italic, " ")
                .or_else(|| self.resolver.resolve_font(&s.family, Script::Latin, s.bold, s.italic))
                .ok_or("no font available to measure Normal style indentation")?;
            let face = ttf_parser::Face::parse(&resolved.font.data, resolved.font.face_index)
                .map_err(|e| format!("Normal style font: {e}"))?;
            if matches!(resolved.metrics, crate::fonts::Metrics::Substitute | crate::fonts::Metrics::Table)
                || resolved.ea_advance.is_some()
                || (s.bold && !face.is_bold())
                || (s.italic && !face.is_italic())
            {
                self.diagnostics.push(diagnostic("window.indentation", format!("Normal style font {:?} uses {:?} face advances for indentation; requested metrics/style may differ", s.family, resolved.family)));
            }
            let advance =
                face.glyph_index(' ').and_then(|g| face.glyph_hor_advance(g)).map(f64::from).unwrap_or_else(|| {
                    self.diagnostics.push(diagnostic(
                        "window.indentation",
                        "Normal style space advance is absent; indentation uses half an em per space",
                    ));
                    f64::from(face.units_per_em()) / 2.0
                });
            let space = advance * s.size / f64::from(face.units_per_em());
            self.normal_space = Some(space);
            space
        };
        // Multiply metrics, never synthesize indentation text or allocate a
        // string proportional to the unsigned SpreadsheetML indent value.
        Ok(f64::from(levels) * 3.0 * space)
    }
    fn font(&mut self, style: &Style, text: &str) -> Result<FontId, String> {
        let script = Script::of(text);
        let key = (style.family.clone(), script, style.bold, style.italic);
        if let Some(ids) = self.keys.get(&key) {
            for id in ids {
                let font = &self.fonts[id.0 as usize];
                if crate::fonts::covers_text(&font.data, font.face_index, text) {
                    return Ok(*id);
                }
            }
        }
        let drawn = self.resolver.resolve_font_for_text(&style.family, script, style.bold, style.italic, text);
        let face = drawn
            .clone()
            .or_else(|| self.resolver.resolve_font(&style.family, Script::Latin, style.bold, style.italic))
            .ok_or("no font available to lay out worksheet text")?;
        if let Some(ids) = self.keys.get(&key) {
            if let Some(id) = ids.iter().find(|id| {
                let font = &self.fonts[id.0 as usize];
                font.face_index == face.font.face_index && font.data == face.font.data
            }) {
                return Ok(*id);
            }
        }
        if face.ea_advance.is_some() {
            self.diagnostics.push(diagnostic(
                format!("window.fonts[{}]", self.fonts.len()),
                "East Asian advance overrides are not applied in the worksheet grid; selected face advances are used",
            ));
        }
        let parsed = ttf_parser::Face::parse(&face.font.data, face.font.face_index)
            .map_err(|e| format!("worksheet font: {e}"))?;
        let id = FontId(self.fonts.len() as u32);
        self.fonts.push(oxml_layout::FontData {
            id,
            family: face.family.clone(),
            data: face.font.data.clone(),
            face_index: face.font.face_index,
            bold: parsed.is_bold(),
            italic: parsed.is_italic(),
        });
        self.faces.insert(
            id,
            FaceInfo {
                requested: style.family.clone(),
                script,
                drawn: drawn.as_ref().map(|f| f.family.clone()),
                source: drawn.as_ref().map(|f| f.source),
                metrics: drawn.map_or(crate::fonts::Metrics::Substitute, |f| {
                    if f.ea_advance.is_some() {
                        crate::fonts::Metrics::Substitute
                    } else {
                        f.metrics
                    }
                }),
            },
        );
        self.keys.entry(key).or_default().push(id);
        Ok(id)
    }
    fn cell(
        &mut self,
        rect: Rect,
        text: &str,
        style: &Style,
        numeric: bool,
        missing_formula: bool,
        path: &str,
    ) -> Result<(), String> {
        self.remaining_text =
            self.remaining_text.checked_sub(text.len()).ok_or("worksheet window exceeds the configured text budget")?;
        if text.chars().any(|c| matches!(c as u32, 0x0300..=0x036F | 0x0590..=0x10FF | 0x1780..=0x17FF | 0x200E..=0x200F | 0x202A..=0x202E | 0x2066..=0x2069)) {
            self.diagnostics.push(diagnostic(format!("{path}.text"), "complex-script shaping and bidirectional reordering are not performed; scalar-positioned text may differ"));
        }
        if let Some(color) = style.fill {
            self.elements.push(PositionedElement::FilledRect { rect, color });
        }
        let corners = [
            (Point { x: rect.x, y: rect.y }, Point { x: rect.x, y: rect.y + rect.height }),
            (Point { x: rect.x + rect.width, y: rect.y }, Point { x: rect.x + rect.width, y: rect.y + rect.height }),
            (Point { x: rect.x, y: rect.y }, Point { x: rect.x + rect.width, y: rect.y }),
            (Point { x: rect.x, y: rect.y + rect.height }, Point { x: rect.x + rect.width, y: rect.y + rect.height }),
        ];
        for (k, (start, end)) in corners.into_iter().enumerate() {
            let (width, color, dash_pattern) =
                style.edges[k].as_ref().map_or((0.3, Color::from_hex("D0D0D0"), None), |e| (e.width, e.color, e.dash));
            self.elements.push(PositionedElement::Line { start, end, width, color, dash_pattern });
        }
        if text.is_empty() {
            return Ok(());
        }
        let guarded = numeric || missing_formula;
        if rect.height <= 0.0 || rect.width <= 0.0 || (rect.width <= 4.0 && !guarded) {
            self.diagnostics.push(diagnostic(
                format!("{path}.clipping"),
                "cell text is omitted because its visible size leaves no text area; cells[].display retains the complete value",
            ));
            return Ok(());
        }
        let indent = self.indentation(style.indent)?;
        let text_width = (rect.width - 4.0 - indent).max(0.0);
        if style.indent != 0 && text_width <= 0.0 && !guarded {
            self.diagnostics.push(diagnostic(format!("{path}.clipping"), "cell text is omitted because its visible size and indentation leave no text area; cells[].display retains the complete value"));
            return Ok(());
        }
        let id = self.font(style, text)?;
        let font = &self.fonts[id.0 as usize];
        let face = ttf_parser::Face::parse(&font.data, font.face_index).map_err(|e| format!("worksheet font: {e}"))?;
        let scale = style.size / f64::from(face.units_per_em());
        if (style.bold && !font.bold) || (style.italic && !font.italic) {
            self.diagnostics.push(diagnostic(
                format!("{path}.font"),
                "requested bold/italic face is unavailable; the selected face is drawn without synthetic styling",
            ));
        }
        let mut lines: Vec<(String, Vec<u16>, Vec<f64>)> = vec![];
        let (mut line, mut glyphs, mut advances, mut width) = (String::new(), vec![], vec![], 0.0);
        for c in text.chars() {
            let glyph = face.glyph_index(c).unwrap_or(ttf_parser::GlyphId(0));
            let advance = f64::from(face.glyph_hor_advance(glyph).unwrap_or(face.units_per_em() / 2)) * scale;
            if c == '\n' || (style.wrap && !guarded && width + advance > text_width && !line.is_empty()) {
                lines.push((std::mem::take(&mut line), std::mem::take(&mut glyphs), std::mem::take(&mut advances)));
                width = 0.0;
            }
            if c != '\n' {
                line.push(c);
                glyphs.push(glyph.0);
                advances.push(advance);
                width += advance;
            }
        }
        lines.push((line, glyphs, advances));
        let line_height = style.size * 1.2;
        let content_height = lines.len() as f64 * line_height;
        let mut clipped = content_height > rect.height || (style.indent != 0 && text_width <= 0.0);
        let top = match style.vertical.as_str() {
            "top" => rect.y + 1.0,
            "center" => rect.y + ((rect.height - content_height) / 2.0).max(0.0),
            _ => rect.y + (rect.height - content_height - 1.0).max(0.0),
        };
        let mut children = vec![];
        for (i, (text, glyph_ids, advances)) in lines.into_iter().enumerate() {
            let width: f64 = advances.iter().sum();
            let x = match style.horizontal.as_str() {
                "center" => rect.x + (rect.width - width) / 2.0,
                "right" => rect.x + rect.width - width - 2.0 - indent,
                "general" if numeric => rect.x + rect.width - width - 2.0,
                _ => rect.x + 2.0 + indent,
            };
            clipped |= x < rect.x || x + width > rect.x + rect.width;
            if guarded {
                let mut pen = x;
                for (&glyph, &advance) in glyph_ids.iter().zip(&advances) {
                    if let Some(bounds) = face.glyph_bounding_box(ttf_parser::GlyphId(glyph)) {
                        let baseline = top + style.size + i as f64 * line_height;
                        clipped |= pen + f64::from(bounds.x_min) * scale < rect.x
                            || pen + f64::from(bounds.x_max) * scale > rect.x + rect.width
                            || baseline - f64::from(bounds.y_max) * scale < rect.y
                            || baseline - f64::from(bounds.y_min) * scale > rect.y + rect.height;
                    }
                    pen += advance;
                }
            }
            children.push(PositionedElement::Text(GlyphRun {
                origin: Point { x, y: top + style.size + i as f64 * line_height },
                font_id: id,
                font_size: style.size,
                glyph_ids,
                advances,
                text,
                source: None,
                color: style.color,
                bold: font.bold,
                italic: font.italic,
                field_kind: None,
                field_source: None,
                note: None,
                tab_aligned: None,
            }));
        }
        if clipped {
            if guarded {
                children = overflow_indicator(
                    rect,
                    &face,
                    id,
                    font.bold,
                    font.italic,
                    style,
                    if missing_formula { '?' } else { '#' },
                );
            }
            self.diagnostics.push(diagnostic(
                format!("{path}.clipping"),
                if numeric {
                    crate::quality::NUMERIC_OVERFLOW
                } else if missing_formula {
                    crate::quality::FORMULA_MARKER_OVERFLOW
                } else {
                    "cell text exceeds its stored size and is clipped; cells[].display retains the complete value"
                },
            ));
        }
        self.elements.push(PositionedElement::Group(GroupElement {
            transform: Transform::IDENTITY,
            clip: Some(Path::rect(rect)),
            opacity: 1.0,
            effects: vec![],
            children,
        }));
        Ok(())
    }
    fn finish(self, width: f64, height: f64) -> Result<Preview, String> {
        let p = Preview {
            image_limits: crate::ImageLimits::default(),
            image_plan: crate::images::ImagePlan::default(),
            layout: LayoutResult::new(
                vec![Arc::new(PageFrame::new(1, width, height, self.elements))],
                self.fonts,
                None,
                vec![],
            ),
            faces: self.faces,
            chars: vec![],
            page_subsets: vec![],
            document_subsets: vec![],
            fonts: FontsReport::default(),
            warnings: self.resolver.warnings().to_vec(),
            diagnostics: self.diagnostics,
        };
        Ok(p)
    }
}

// Replace the whole value. Fit the indicator's actual ink bounds, even for
// short rows/tiny visible columns; never leave a readable numeric suffix.
fn overflow_indicator(
    rect: Rect,
    face: &ttf_parser::Face<'_>,
    id: FontId,
    bold: bool,
    italic: bool,
    style: &Style,
    marker: char,
) -> Vec<PositionedElement> {
    if let Some(glyph) = face.glyph_index(marker) {
        if let (Some(advance), Some(bounds)) = (face.glyph_hor_advance(glyph), face.glyph_bounding_box(glyph)) {
            let upem = f64::from(face.units_per_em());
            let count =
                if marker == '#' && f64::from(advance) * style.size / upem * 3.0 <= rect.width - 4.0 { 3 } else { 1 };
            let ink_width = f64::from(bounds.x_max) - f64::from(bounds.x_min) + (count - 1) as f64 * f64::from(advance);
            let ink_height = f64::from(bounds.y_max) - f64::from(bounds.y_min);
            if ink_width > 0.0 && ink_height > 0.0 {
                let scale = (style.size / upem).min(rect.width * 0.8 / ink_width).min(rect.height * 0.8 / ink_height);
                return vec![PositionedElement::Text(GlyphRun {
                    origin: Point {
                        x: rect.x + (rect.width - ink_width * scale) / 2.0 - f64::from(bounds.x_min) * scale,
                        y: rect.y + (rect.height - ink_height * scale) / 2.0 + f64::from(bounds.y_max) * scale,
                    },
                    font_id: id,
                    font_size: scale * upem,
                    glyph_ids: vec![glyph.0; count],
                    advances: vec![f64::from(advance) * scale; count],
                    text: std::iter::repeat_n(marker, count).collect(),
                    source: None,
                    color: style.color,
                    bold,
                    italic,
                    field_kind: None,
                    field_source: None,
                    note: None,
                    tab_aligned: None,
                })];
            }
        }
    }
    // A drawing font can lack ASCII markers. A vector cross cannot be read as
    // a truncated number and does not depend on missing font coverage.
    [(0.2, 0.2, 0.8, 0.8), (0.2, 0.8, 0.8, 0.2)]
        .into_iter()
        .map(|(x1, y1, x2, y2)| PositionedElement::Line {
            start: Point { x: rect.x + rect.width * x1, y: rect.y + rect.height * y1 },
            end: Point { x: rect.x + rect.width * x2, y: rect.y + rect.height * y2 },
            width: (rect.width.min(rect.height) * 0.1).min(1.0),
            color: style.color,
            dash_pattern: None,
        })
        .collect()
}
