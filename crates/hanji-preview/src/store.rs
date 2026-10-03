//! The preview of a stored document, for the CLI: the
//! revision is exported in memory (§2 rule 4: the bytes export would
//! write; the original file is never read again), rendered, and written as
//! HTML, SVG or PNG files.

use std::io::Read as _;
use std::path::{Path, PathBuf};

use hanji_store::{Code, Error, ExportOptions, Format, Result, Storage, Workspace};
use serde::Serialize;

use crate::{render_pptx, xlsx, Diagnostic, FontsReport, Options, PageData, PageFormat, PageInfo, Preview};

/// What the preview writes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Output {
    /// One self-contained viewer file.
    #[default]
    Html,
    /// One SVG per slide or worksheet window.
    Svg,
    /// One PNG per slide or worksheet window (96 DPI).
    Png,
}

impl Output {
    pub fn parse(s: &str) -> Option<Output> {
        match s.trim().to_lowercase().as_str() {
            "html" => Some(Output::Html),
            "svg" => Some(Output::Svg),
            "png" => Some(Output::Png),
            _ => None,
        }
    }
}

/// The PNG resolution: 1 px per CSS pixel.
pub const PNG_DPI: f64 = 96.0;

#[derive(Clone, Debug, Serialize)]
pub struct Previewed {
    pub doc_id: String,
    pub revision: u32,
    pub format: Format,
    pub output: Output,
    pub slides: usize,
    /// The files written, in slide order.
    pub files: Vec<String>,
    pub fonts: FontsReport,
    /// The fonts report in one line.
    pub summary: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    /// Rendering and font-embedding fallbacks, with their source paths.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

/// Native adapter selection. Worksheet indexes here are 1-based, as in the CLI;
/// the portable workbook API and serialized `SheetInfo.index` remain 0-based.
#[derive(Clone, Debug, Default)]
pub struct XlsxSelection {
    pub sheet: Option<String>,
    pub sheet_index: Option<usize>,
    pub range: Option<String>,
}

impl XlsxSelection {
    pub fn is_requested(&self) -> bool {
        self.sheet.is_some() || self.sheet_index.is_some() || self.range.is_some()
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum XlsxSource {
    File,
    Revision,
}

/// One bounded worksheet window, with the same reports as the portable API.
#[derive(Clone, Debug, Serialize)]
pub struct XlsxPreviewed {
    pub doc_id: String,
    pub revision: u32,
    pub source_kind: XlsxSource,
    pub format: Format,
    pub output: Output,
    pub files: Vec<String>,
    pub sheet: xlsx::SheetInfo,
    pub sheets: Vec<xlsx::SheetInfo>,
    pub range: String,
    pub page: PageInfo,
    pub cells: Vec<xlsx::CellInfo>,
    pub fonts: FontsReport,
    pub summary: String,
    pub warnings: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

fn render_xlsx_bytes(
    bytes: &[u8],
    id: String,
    revision: u32,
    source_kind: XlsxSource,
    selection: &XlsxSelection,
    opts: &Options,
) -> Result<(XlsxPreviewed, xlsx::Window)> {
    if selection.sheet.is_some() && selection.sheet_index.is_some() {
        return Err(Error::bad("choose --sheet NAME or --sheet-index N, not both"));
    }
    let mut book = xlsx::open_xlsx(bytes, xlsx::XlsxOptions::default())
        .map_err(|m| Error::new(Code::Package, format!("cannot preview {id}: {m}")))?;
    let index = if let Some(name) = &selection.sheet {
        book.sheets.iter().position(|s| &s.name == name).ok_or_else(|| {
            Error::bad(format!(
                "worksheet {name:?} not found; available sheets: {}",
                book.sheets.iter().map(|s| format!("{:?}", s.name)).collect::<Vec<_>>().join(", ")
            ))
        })?
    } else if let Some(index) = selection.sheet_index {
        index
            .checked_sub(1)
            .filter(|&i| i < book.sheets.len())
            .ok_or_else(|| Error::bad(format!("--sheet-index is 1-based; choose 1 through {}", book.sheets.len())))?
    } else {
        book.sheets.iter().position(|s| s.is_worksheet && s.state == "visible").ok_or_else(|| {
            Error::bad("no visible worksheet; select a worksheet explicitly with --sheet or --sheet-index")
        })?
    };
    if !book.sheets[index].is_worksheet {
        return Err(Error::new(Code::Unsupported, "chart/macro/dialog sheets do not have a worksheet grid preview"));
    }
    let window = book
        .render_window(index, selection.range.as_deref().unwrap_or("A1:L40"), opts)
        .map_err(|m| Error::bad(format!("cannot preview worksheet {:?}: {m}", book.sheets[index].name)))?;
    let out = XlsxPreviewed {
        doc_id: id,
        revision,
        source_kind,
        format: Format::Xlsx,
        output: Output::Html,
        files: vec![],
        sheet: window.sheet.clone(),
        sheets: book.sheets,
        range: window.range.clone(),
        page: window.page_info(),
        cells: window.cells.clone(),
        fonts: window.fonts().clone(),
        summary: window.fonts().summary(),
        warnings: window.warnings().to_vec(),
        diagnostics: window.diagnostics().to_vec(),
    };
    Ok((out, window))
}

/// Read the file without importing, recalculating or storing it. The raw-file
/// read and unpacked package each have a 64 MiB budget in this native adapter.
pub fn render_xlsx_file(
    path: &Path,
    selection: &XlsxSelection,
    opts: &Options,
) -> Result<(XlsxPreviewed, xlsx::Window)> {
    let limit = xlsx::XlsxOptions::default().max_unpacked_bytes;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|f| f.take(limit + 1).read_to_end(&mut bytes))
        .map_err(|e| Error::io(format!("cannot read {}: {e}", path.display())))?;
    if bytes.len() as u64 > limit {
        return Err(Error::bad("XLSX preview file exceeds the 64 MiB raw-file budget"));
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    render_xlsx_bytes(&bytes, hanji_store::id_from_name(&name), 1, XlsxSource::File, selection, opts)
}

/// Preview the bytes exported from a stored revision, never its original file.
pub fn render_xlsx<S: Storage>(
    ws: &Workspace<S>,
    id: &str,
    revision: Option<u32>,
    selection: &XlsxSelection,
    opts: &Options,
) -> Result<(XlsxPreviewed, xlsx::Window)> {
    if ws.doc(id)?.format != Format::Xlsx {
        return Err(Error::bad("worksheet selection requires an xlsx document"));
    }
    let (e, bytes) =
        ws.export_bytes(id, revision, &ExportOptions { acknowledge_surfaced: true, tracked_changes: false })?;
    render_xlsx_bytes(&bytes, e.doc_id, e.revision, XlsxSource::Revision, selection, opts)
}

/// Render before creating output files, so window/raster budget refusals do not
/// leave an empty output directory. Names distinguish sheet and canonical range.
pub fn write_xlsx(window: &xlsx::Window, out: &mut XlsxPreviewed, output: Output, dir: &Path) -> Result<()> {
    let stem =
        format!("{}-r{}-sheet-{}-{}", out.doc_id, out.revision, out.sheet.index + 1, out.range.replace(':', "-"));
    let (ext, bytes) = match output {
        Output::Html => {
            let title = format!("{} · revision {} · {} · {}", out.doc_id, out.revision, out.sheet.name, out.range);
            ("html", window.html(&title).into_bytes())
        }
        Output::Svg | Output::Png => {
            let format = if output == Output::Svg { PageFormat::Svg } else { PageFormat::Png { dpi: PNG_DPI } };
            match window.render(format).map_err(|m| Error::bad(format!("cannot render worksheet window: {m}")))?.data {
                PageData::Svg(svg) => ("svg", svg.into_bytes()),
                PageData::Png(png) => ("png", png),
            }
        }
    };
    std::fs::create_dir_all(dir).map_err(|e| Error::io(format!("cannot make {}: {e}", dir.display())))?;
    let path = dir.join(format!("{stem}.{ext}"));
    std::fs::write(&path, bytes).map_err(|e| Error::io(format!("cannot write {}: {e}", path.display())))?;
    out.output = output;
    out.files = vec![std::path::absolute(&path).unwrap_or(path).display().to_string()];
    Ok(())
}

/// Whether hanji can preview `format` yet, as the refusal to give.
pub fn check_format(format: Format) -> Result<()> {
    match format {
        Format::Pptx | Format::Xlsx => Ok(()),
        f => Err(Error::new(
            Code::Unsupported,
            format!(
                "preview not supported yet for {}: hanji previews pptx slides and xlsx worksheet windows (docx and hwpx are planned).",
                f.name()
            ),
        )),
    }
}

/// Renders revision `revision` (the current one by default) of `id`.
pub fn render<S: Storage>(
    ws: &Workspace<S>,
    id: &str,
    revision: Option<u32>,
    opts: &Options,
) -> Result<(Previewed, Preview)> {
    let doc = ws.doc(id)?;
    check_format(doc.format)?;
    if doc.format != Format::Pptx {
        return Err(Error::bad("use render_xlsx for a worksheet window"));
    }
    // The preview stays on this machine: content to surface does not stop it.
    let (e, bytes) =
        ws.export_bytes(id, revision, &ExportOptions { acknowledge_surfaced: true, tracked_changes: false })?;
    let p = render_pptx(&bytes, opts).map_err(|m| Error::new(Code::Package, format!("cannot preview {id}: {m}")))?;
    let out = Previewed {
        doc_id: e.doc_id,
        revision: e.revision,
        format: e.format,
        output: Output::Html,
        slides: p.slide_count(),
        files: vec![],
        summary: p.fonts.summary(),
        fonts: p.fonts.clone(),
        warnings: p.warnings.clone(),
        diagnostics: p.diagnostics.clone(),
    };
    Ok((out, p))
}

/// Where a document's preview goes by default: beside the file it was opened
/// from, else `fallback`.
pub fn default_dir<S: Storage>(ws: &Workspace<S>, id: &str, fallback: &Path) -> PathBuf {
    ws.doc(id)
        .ok()
        .and_then(|d| d.source)
        .and_then(|s| Path::new(&s).parent().map(Path::to_path_buf))
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| fallback.to_path_buf())
}

/// Writes `p` as `output` into `dir`: `<doc>-r<rev>-preview.html`, or
/// `<doc>-r<rev>-slide-<n>.svg` / `.png`.
pub fn write(p: &Preview, out: &mut Previewed, output: Output, dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|e| Error::io(format!("cannot make {}: {e}", dir.display())))?;
    let stem = format!("{}-r{}", out.doc_id, out.revision);
    let put = |name: String, data: &[u8]| -> Result<String> {
        let path = dir.join(name);
        std::fs::write(&path, data).map_err(|e| Error::io(format!("cannot write {}: {e}", path.display())))?;
        Ok(std::path::absolute(&path).unwrap_or(path).display().to_string())
    };
    out.output = output;
    out.files = match output {
        Output::Html => {
            let title = format!("{} · revision {} · preview", out.doc_id, out.revision);
            vec![put(format!("{stem}-preview.html"), p.html(&title).as_bytes())?]
        }
        Output::Svg => (0..p.slide_count())
            .map(|k| put(format!("{stem}-slide-{}.svg", k + 1), p.slide_svg(k).as_bytes()))
            .collect::<Result<_>>()?,
        Output::Png => (0..p.slide_count())
            .map(|k| {
                let png =
                    p.slide_png(k, PNG_DPI).map_err(|m| Error::new(Code::Package, format!("slide {}: {m}", k + 1)))?;
                put(format!("{stem}-slide-{}.png", k + 1), &png)
            })
            .collect::<Result<_>>()?,
    };
    Ok(())
}
