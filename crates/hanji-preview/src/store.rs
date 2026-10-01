//! The preview of a stored document, for the CLI and the MCP server: the
//! revision is exported in memory (§2 rule 4: the bytes export would
//! write; the original file is never read again), rendered, and written as
//! HTML, SVG or PNG files.

use std::path::{Path, PathBuf};

use hanji_store::{Code, Error, ExportOptions, Format, Result, Storage, Workspace};
use serde::Serialize;

use crate::{render_pptx, FontsReport, Options, Preview};

/// What the preview writes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Output {
    /// One self-contained viewer file.
    #[default]
    Html,
    /// One SVG per slide.
    Svg,
    /// One PNG per slide (96 DPI).
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
}

/// Whether hanji can preview `format` yet, as the refusal to give.
pub fn check_format(format: Format) -> Result<()> {
    match format {
        Format::Pptx => Ok(()),
        f => Err(Error::new(
            Code::Unsupported,
            format!(
                "preview not supported yet for {}: hanji previews pptx only for now (docx and hwpx are planned, xlsx later).",
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
