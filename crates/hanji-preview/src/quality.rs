//! Additive quality contract over the existing path/message diagnostics.
//!
//! Legacy render methods and diagnostic struct literals remain unchanged. The
//! report describes implemented checks, never native-application completeness.
use serde::Serialize;

use crate::{Diagnostic, DocumentKind, FontsReport};

pub(crate) const NUMERIC_OVERFLOW: &str = "numeric/date value does not fit its stored cell size; an overflow indicator replaces the entire visible value; cells[].display retains the complete value";
pub(crate) const FORMULA_MISSING: &str =
    "formula has no cached result; shown as #UNEVALUATED; no evaluation was performed";
pub(crate) const FORMULA_MARKER_OVERFLOW: &str = "missing formula cache marker does not fit its stored cell size; a question-mark indicator replaces the visible marker; cells[].display retains #UNEVALUATED";
pub(crate) const GENERIC_FONT: &str = "selected font provides generic LastResort symbols, not character-specific glyphs; source text is preserved but character coverage is unavailable";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PreviewSource {
    Pptx,
    Docx,
    Hwpx,
    Xlsx,
}
impl From<DocumentKind> for PreviewSource {
    fn from(value: DocumentKind) -> Self {
        match value {
            DocumentKind::Pptx => Self::Pptx,
            DocumentKind::Docx => Self::Docx,
            DocumentKind::Hwpx => Self::Hwpx,
        }
    }
}

/// Stable identifiers. Consumers should tolerate additional codes in later versions.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum DiagnosticCode {
    #[serde(rename = "renderer.layout-report")]
    LayoutReport,
    #[serde(rename = "svg.text-approximation")]
    TextApproximation,
    #[serde(rename = "svg.invalid-text-replaced")]
    InvalidText,
    #[serde(rename = "svg.invalid-positioning-omitted")]
    InvalidPositioning,
    #[serde(rename = "svg.element-omitted")]
    ElementOmitted,
    #[serde(rename = "svg.image-omitted")]
    ImageOmitted,
    #[serde(rename = "svg.image-resource-limit")]
    ImageResourceLimit,
    #[serde(rename = "svg.link-omitted")]
    LinkOmitted,
    #[serde(rename = "svg.paint-omitted")]
    PaintOmitted,
    #[serde(rename = "svg.effect-omitted")]
    EffectOmitted,
    #[serde(rename = "svg.effect-approximation")]
    EffectApproximation,
    #[serde(rename = "font.embedding-failed")]
    FontEmbeddingFailed,
    #[serde(rename = "font.data-missing")]
    FontDataMissing,
    #[serde(rename = "font.face-mismatch")]
    FontFaceMismatch,
    #[serde(rename = "font.missing-glyphs")]
    MissingGlyphs,
    #[serde(rename = "font.substituted")]
    FontSubstituted,
    #[serde(rename = "font.generic-symbols")]
    FontGenericSymbols,
    #[serde(rename = "xlsx.grid-approximation")]
    GridApproximation,
    #[serde(rename = "xlsx.numeric-overflow")]
    NumericOverflow,
    #[serde(rename = "xlsx.formula-marker-overflow")]
    FormulaMarkerOverflow,
    #[serde(rename = "xlsx.text-clipped")]
    TextClipped,
    #[serde(rename = "xlsx.text-omitted")]
    TextOmitted,
    #[serde(rename = "xlsx.formula-cache-missing")]
    FormulaCacheMissing,
    #[serde(rename = "xlsx.formula-cache-unverified")]
    FormulaCacheUnverified,
    #[serde(rename = "xlsx.formula-cache-stale")]
    FormulaCacheStale,
    #[serde(rename = "xlsx.feature-unsupported")]
    WorksheetFeatureUnsupported,
    #[serde(rename = "xlsx.style-unsupported")]
    WorksheetStyleUnsupported,
    #[serde(rename = "xlsx.text-approximation")]
    WorksheetTextApproximation,
    #[serde(rename = "hwpx.image-omitted")]
    HwpxImageOmitted,
    #[serde(rename = "hwpx.table-overflow")]
    TableOverflow,
    #[serde(rename = "hwpx.table-overlap")]
    TableOverlap,
    #[serde(rename = "preview.native-fidelity-unverified")]
    NativeFidelityUnverified,
    #[serde(rename = "preview.warning-unclassified")]
    UnclassifiedWarning,
    #[serde(rename = "preview.diagnostic-unclassified")]
    UnclassifiedDiagnostic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Consequence {
    Approximation,
    UnsupportedOmission,
    MissingText,
    ValueUnavailable,
    OverflowIndicator,
    Clipping,
    FontSubstitution,
    Unverified,
    Unclassified,
}

/// Indices are zero-based, including font-report page indices translated here.
/// Absent indices mean the source diagnostic did not establish that location.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DiagnosticLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_index: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub page_indices: Vec<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct QualityDiagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub source: PreviewSource,
    pub location: DiagnosticLocation,
    pub consequence: Consequence,
    /// Original strings are retained for existing path/message consumers.
    pub path: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum QualityStatus {
    NoKnownIssues,
    Degraded,
    UnsupportedContent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RenderingStatus {
    Ready,
    Rendered,
    Refused,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiagnosticCoverage {
    /// Always false in v1: no detector proves every document feature/fidelity.
    pub complete: bool,
    pub checked: Vec<&'static str>,
    pub unchecked: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct QualityReport {
    pub version: u32,
    pub source: PreviewSource,
    pub rendering: RenderingStatus,
    pub status: QualityStatus,
    pub coverage: DiagnosticCoverage,
    pub diagnostics: Vec<QualityDiagnostic>,
}

/// Opt-in rejection only for detected critical losses, not general warnings,
/// experimental status, unknown upstream reports, or unproven completeness.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Strictness {
    #[default]
    AllowKnownLosses,
    CriticalLosses,
}

#[derive(Clone, Debug, Serialize)]
pub struct QualityRefusal {
    pub code: &'static str,
    pub quality: QualityReport,
}
impl std::fmt::Display for QualityRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "preview refused: detected critical loss; inspect quality.diagnostics")
    }
}
impl std::error::Error for QualityRefusal {}

/// Flatten an unchanged legacy result and add `quality` in machine output.
#[derive(Serialize)]
pub struct WithQuality<'a, T: Serialize> {
    #[serde(flatten)]
    pub result: &'a T,
    pub quality: QualityReport,
}

fn bracket_index(path: &str, prefix: &str) -> Option<usize> {
    path.strip_prefix(prefix)?.split_once(']')?.0.parse().ok()
}
fn location(path: &str) -> DiagnosticLocation {
    let cell = path.split_once(".cells[").and_then(|(_, s)| s.split_once(']')).map(|(s, _)| s.to_owned());
    // Report-array indices and aggregate HWPX table/image counters do not
    // identify a document object. Preserve their legacy path without inventing one.
    let object_known = cell.is_some()
        || path.contains(".elements[")
        || path.ends_with(".background")
        || path.contains(".styles[")
        || path.contains(".fonts[")
        || path.starts_with("fonts[");
    DiagnosticLocation {
        page_index: bracket_index(path, "pages["),
        sheet_index: bracket_index(path, "sheets["),
        cell,
        object_path: object_known.then(|| path.to_owned()),
        ..Default::default()
    }
}

// Translate known owned emissions and structural paths. Opaque upstream
// messages remain unclassified reports; keyword guesses never make them critical.
fn classify(source: PreviewSource, d: &Diagnostic) -> (DiagnosticCode, Severity, Consequence) {
    use Consequence::*;
    use DiagnosticCode::*;
    use Severity::*;
    if d.path.ends_with(".subset") {
        return (FontEmbeddingFailed, Error, MissingText);
    }
    if d.path.starts_with("layout.diagnostics[") {
        return (LayoutReport, Warning, Unclassified);
    }
    if source == PreviewSource::Xlsx {
        if d.message == NUMERIC_OVERFLOW {
            return (NumericOverflow, Warning, OverflowIndicator);
        }
        if d.message == FORMULA_MARKER_OVERFLOW {
            return (FormulaMarkerOverflow, Warning, OverflowIndicator);
        }
        if d.path.ends_with(".formula") {
            return if d.message == FORMULA_MISSING {
                (FormulaCacheMissing, Error, ValueUnavailable)
            } else if d.message.starts_with("stored formula result shown; workbook requests recalculation") {
                (FormulaCacheStale, Warning, Unverified)
            } else if d.message.starts_with("stored formula result shown; cache freshness is unverified;") {
                (FormulaCacheUnverified, Warning, Unverified)
            } else {
                (UnclassifiedDiagnostic, Warning, Unclassified)
            };
        }
        if d.path.ends_with(".clipping") {
            if d.path.starts_with("header.") {
                return (TextClipped, Warning, Clipping);
            }
            return if d.message.starts_with("cell text is omitted because its visible size") {
                (TextOmitted, Error, MissingText)
            } else {
                (TextClipped, Warning, Clipping)
            };
        }
        if d.path.ends_with(".grid") {
            return (GridApproximation, Warning, Approximation);
        }
        if d.path.contains(".styles[") {
            return (WorksheetStyleUnsupported, Warning, Approximation);
        }
        if d.path.ends_with(".text")
            || d.path.ends_with(".font")
            || d.path.starts_with("window.fonts[")
            || d.path == "window.indentation"
        {
            return (WorksheetTextApproximation, Warning, Approximation);
        }
        if d.path == "workbook.macros" || d.path == "workbook.externalLinks" {
            return (WorksheetFeatureUnsupported, Info, Unverified);
        }
        if [
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
        ]
        .iter()
        .any(|feature| d.path.ends_with(&format!(".{feature}")))
        {
            return (WorksheetFeatureUnsupported, Warning, UnsupportedOmission);
        }
    }
    if source == PreviewSource::Hwpx {
        if d.message == "external image reference omitted; preview does not fetch external resources" {
            return (HwpxImageOmitted, Error, UnsupportedOmission);
        }
        if d.path.ends_with(".tables") && d.message.starts_with("rhwp reports ") {
            if d.message.contains("overflowing cell lines") {
                return (TableOverflow, Warning, Clipping);
            }
            if d.message.ends_with("table overlaps") {
                return (TableOverlap, Warning, Approximation);
            }
        }
    }
    match d.message.as_str() {
        GENERIC_FONT => (FontGenericSymbols, Error, MissingText),
        "image bytes are neither PNG nor JPEG and were omitted" => (ImageOmitted, Error, UnsupportedOmission),
        "embedded image exceeds the configured decoded-image byte budget and was omitted"
        | "embedded image exceeds the configured page decoded-image byte budget and was omitted"
        | "embedded image exceeds the configured document decoded-image byte budget and was omitted"
        | "embedded image decoded bytes cannot be represented on this target and it was omitted"
        | "embedded image page decoded-byte total overflowed and the image was omitted"
        | "embedded image document decoded-byte total overflowed and the image was omitted"
        | "embedded image dimensions overflow the decoded RGBA byte count and the image was omitted"
        | "embedded image dimensions exceed the raster backend representation and the image was omitted" =>
            (ImageResourceLimit, Error, UnsupportedOmission),
        "embedded image dimensions must be positive and the image was omitted"
        | "embedded image PNG dimensions could not be read and the image was omitted"
        | "embedded image JPEG dimensions could not be read and the image was omitted" =>
            (ImageOmitted, Error, UnsupportedOmission),
        "unsupported positioned element was omitted from SVG output" => (ElementOmitted, Error, UnsupportedOmission),
        "invalid multilingual glyph positioning was omitted from SVG output" => (InvalidPositioning, Error, MissingText),
        "text references font data that is absent from the layout result" => (FontDataMissing, Error, MissingText),
        "font collection face index cannot be selected by SVG and uses the default face" => (FontFaceMismatch, Warning, Approximation),
        "XML-invalid text characters were replaced with U+FFFD" => (InvalidText, Error, MissingText),
        "complex shaping kept searchable text with total-advance positioning" | "multilingual shaping kept searchable text with browser-positioned glyph approximation" => (TextApproximation, Warning, Approximation),
        "active or unsupported link target was omitted" => (LinkOmitted, Warning, UnsupportedOmission),
        "tile paint was omitted because its carrier has no media bytes" => (PaintOmitted, Error, UnsupportedOmission),
        "unsupported group effect was omitted while its children were preserved" | "group effects were omitted because singular-transform source bounds could not be proven" => (EffectOmitted, Warning, UnsupportedOmission),
        "non-uniform or skewed transform makes SVG shadow blur anisotropic instead of the raster backend's average-scale isotropic blur" | "asymmetric or disabled gradient extension uses SVG pad extension" => (EffectApproximation, Warning, Approximation),
        _ => (UnclassifiedDiagnostic, Warning, Unclassified),
    }
}

impl QualityReport {
    /// Inspect preflight evidence without implying an output was written.
    pub fn inspect(source: PreviewSource, legacy: &[Diagnostic], fonts: &FontsReport, warnings: &[String]) -> Self {
        let mut diagnostics: Vec<_> = legacy
            .iter()
            .map(|d| {
                let (code, severity, consequence) = classify(source, d);
                QualityDiagnostic {
                    code,
                    severity,
                    source,
                    location: location(&d.path),
                    consequence,
                    path: d.path.clone(),
                    message: d.message.clone(),
                }
            })
            .collect();
        for missing in &fonts.missing_glyphs {
            let pages: Vec<_> = missing.pages.iter().filter_map(|p| p.checked_sub(1)).collect();
            diagnostics.push(QualityDiagnostic {
                code: DiagnosticCode::MissingGlyphs,
                severity: Severity::Error,
                source,
                location: DiagnosticLocation {
                    page_index: if pages.len() == 1 { Some(pages[0]) } else { None },
                    page_indices: pages,
                    ..Default::default()
                },
                consequence: Consequence::MissingText,
                path: "fonts.missing_glyphs".into(),
                message: format!(
                    "{} is unavailable in the drawing font requested as {:?}",
                    missing.char, missing.requested
                ),
            });
        }
        if fonts.missing_glyphs_total > fonts.missing_glyphs.len() {
            diagnostics.push(QualityDiagnostic {
                code: DiagnosticCode::MissingGlyphs,
                severity: Severity::Error,
                source,
                location: DiagnosticLocation::default(),
                consequence: Consequence::MissingText,
                path: "fonts.missing_glyphs_total".into(),
                message: format!(
                    "{} distinct missing characters; the legacy list is bounded",
                    fonts.missing_glyphs_total
                ),
            });
        }
        for substitution in &fonts.substituted {
            let pages: Vec<_> = substitution.pages.iter().filter_map(|p| p.checked_sub(1)).collect();
            diagnostics.push(QualityDiagnostic {
                code: DiagnosticCode::FontSubstituted,
                severity: Severity::Warning,
                source,
                location: DiagnosticLocation {
                    page_index: if pages.len() == 1 { Some(pages[0]) } else { None },
                    page_indices: pages,
                    ..Default::default()
                },
                consequence: Consequence::FontSubstitution,
                path: "fonts.substituted".into(),
                message: format!(
                    "font {:?} drawn as {:?}; {} characters",
                    substitution.requested, substitution.drawn, substitution.chars
                ),
            });
        }
        for warning in warnings {
            if legacy.iter().any(|d| warning.contains(&d.message)) {
                continue;
            }
            let experimental = warning.starts_with("DOCX preview is experimental:")
                || warning.starts_with("HWPX preview is experimental:");
            diagnostics.push(QualityDiagnostic {
                code: if experimental {
                    DiagnosticCode::NativeFidelityUnverified
                } else {
                    DiagnosticCode::UnclassifiedWarning
                },
                severity: if experimental { Severity::Info } else { Severity::Warning },
                source,
                location: DiagnosticLocation::default(),
                consequence: if experimental { Consequence::Unverified } else { Consequence::Unclassified },
                path: "warnings".into(),
                message: warning.clone(),
            });
        }
        let status = if diagnostics.iter().any(|d| d.consequence == Consequence::UnsupportedOmission) {
            QualityStatus::UnsupportedContent
        } else if diagnostics.iter().any(|d| d.severity != Severity::Info) {
            QualityStatus::Degraded
        } else {
            QualityStatus::NoKnownIssues
        };
        let mut checked = vec!["reported-font-character-coverage", "font-subset-embedding"];
        let mut unchecked = vec!["native-application-fidelity", "unreported-renderer-losses"];
        match source {
            PreviewSource::Pptx | PreviewSource::Docx => {
                checked.extend(["svg-lowering", "upstream-layout-reports"]);
                unchecked.push("exhaustive-document-feature-detection");
            }
            PreviewSource::Hwpx => {
                checked.extend(["external-image-references", "reported-table-overflow", "reported-table-overlap"]);
                unchecked.extend(["exhaustive-hancom-control-detection", "exact-font-reflow"]);
            }
            PreviewSource::Xlsx => {
                checked.extend([
                    "numeric-cell-fit",
                    "stored-formula-cache-presence",
                    "workbook-recalculation-flags",
                    "listed-worksheet-feature-presence",
                ]);
                unchecked.extend([
                    "formula-calculation-and-cache-freshness",
                    "excel-print-layout",
                    "exhaustive-worksheet-feature-detection",
                ]);
            }
        }
        Self {
            version: 1,
            source,
            rendering: RenderingStatus::Ready,
            status,
            coverage: DiagnosticCoverage { complete: false, checked, unchecked },
            diagnostics,
        }
    }

    /// Mark only after the adapter successfully produced the requested output.
    pub fn rendered(mut self) -> Self {
        self.rendering = RenderingStatus::Rendered;
        self
    }

    /// Call before rendering/writing when detected critical loss must refuse.
    /// A successful check does not assert complete detection or native fidelity.
    pub fn enforce(&self, strictness: Strictness) -> Result<(), QualityRefusal> {
        if strictness == Strictness::CriticalLosses && self.diagnostics.iter().any(|d| d.severity == Severity::Error) {
            let mut quality = self.clone();
            quality.rendering = RenderingStatus::Refused;
            return Err(QualityRefusal { code: "preview.critical-loss-refused", quality });
        }
        Ok(())
    }

    pub fn summary(&self) -> &'static str {
        match self.status {
            QualityStatus::NoKnownIssues => "no-known-issues",
            QualityStatus::Degraded => "degraded",
            QualityStatus::UnsupportedContent => "unsupported-content",
        }
    }

    pub(crate) fn html_notice(&self) -> String {
        format!("<p class=\"hanji-quality\">Preview quality: {}. Diagnostic coverage is partial; native-application completeness and fidelity are unverified.</p>", self.summary())
    }
}

pub(crate) fn add_html_notice(mut html: String, quality: &QualityReport) -> String {
    if let Some(offset) = html.find("<header>") {
        html.insert_str(offset + "<header>".len(), &quality.html_notice());
    } else if let Some(offset) = html.find("<body>") {
        html.insert_str(offset + "<body>".len(), &quality.html_notice());
    }
    html
}
