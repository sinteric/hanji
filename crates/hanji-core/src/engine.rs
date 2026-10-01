//! The engine interface (§7): every format engine sits behind it.

use crate::model::Capabilities;
use crate::remainder::Remainder;
use hanji_format::Diagnostic;

/// Something neutralised or to be surfaced (§8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    /// e.g. `external-template`, `dde-field`, `ole-object`, `macros`, `hidden-text`, `comment`.
    pub kind: String,
    /// Where: a part name, or a paragraph and its text.
    pub location: String,
    pub detail: String,
}

/// Add a notice to a report list.
pub fn notice(report: &mut Vec<Notice>, kind: &str, location: impl Into<String>, detail: impl Into<String>) {
    report.push(Notice { kind: kind.into(), location: location.into(), detail: detail.into() });
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    /// Active or remote content removed on import; it is not preserved.
    pub neutralised: Vec<Notice>,
    /// Content to show a person before export: hidden text, comments,
    /// tracked deletions, author metadata.
    pub surface: Vec<Notice>,
}

#[derive(Clone, Debug)]
pub struct Imported {
    pub text: String,
    pub remainder: Remainder,
    pub report: ImportReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineError {
    /// The package cannot be read (not a zip, missing part, malformed XML, …).
    Package(String),
    /// The model text is not valid for this file.
    Invalid(Vec<Diagnostic>),
    /// Export cannot place something; nothing is dropped silently.
    Refused(String),
    Unsupported(&'static str),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Package(m) => write!(f, "package: {}", hanji_format::chars::name_in(m)),
            EngineError::Invalid(d) => write!(f, "{}", hanji_format::diag::render(d)),
            EngineError::Refused(m) => write!(f, "refused: {}", hanji_format::chars::name_in(m)),
            EngineError::Unsupported(m) => write!(f, "unsupported: {m}"),
        }
    }
}

impl std::error::Error for EngineError {}

#[derive(Clone, Debug)]
pub struct ImportOptions {
    /// §8: neutralise active and remote content. Only tests turn it off.
    pub neutralise: bool,
    /// `template:` written into the front matter.
    pub template: Option<String>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        ImportOptions { neutralise: true, template: None }
    }
}

/// A rendered preview (a stub: the pptx preview is the native `hanji-preview`
/// crate, since it reads font files).
#[derive(Clone, Debug)]
pub struct Rendered {
    pub pages: Vec<Vec<u8>>,
    pub media_type: &'static str,
}

/// Import, export and preview for one home format. Bytes in, bytes out: no
/// filesystem, network or thread I/O.
pub trait Engine {
    fn format(&self) -> &'static str;
    fn capabilities(&self) -> Capabilities;
    /// package → model text + remainder.
    fn import(&self, package: &[u8], opts: &ImportOptions) -> Result<Imported, EngineError>;
    /// model text + remainder (of the same revision) → package.
    fn export(&self, text: &str, remainder: &Remainder) -> Result<Vec<u8>, EngineError>;
    /// Renders the bytes `export` produced (§2 rule 4). Not implemented by the
    /// engines (see [`Rendered`]).
    fn render(&self, _package: &[u8]) -> Result<Rendered, EngineError> {
        Err(EngineError::Unsupported("rendering is not implemented yet"))
    }
}
