//! The office types and home formats, and what each format's engine and
//! text grammar give the operations.

use hanji_core::{Block, Capabilities, DocumentModel, Engine, Remainder, TextModel};
use hanji_docx::DocxEngine;
use hanji_format::{Diagnostic, Names};
use hanji_hwpx::HwpxEngine;
use hanji_pptx::{PptxEngine, PptxModel};
use hanji_xlsx::XlsxEngine;
use serde::{Deserialize, Serialize};

/// The office type (DESIGN.md §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocType {
    Document,
    Presentation,
    Spreadsheet,
}

impl DocType {
    pub const ALL: [DocType; 3] = [DocType::Document, DocType::Presentation, DocType::Spreadsheet];

    pub fn name(self) -> &'static str {
        match self {
            DocType::Document => "document",
            DocType::Presentation => "presentation",
            DocType::Spreadsheet => "spreadsheet",
        }
    }

    pub fn parse(s: &str) -> Option<DocType> {
        DocType::ALL.into_iter().find(|t| t.name() == s.trim().to_lowercase())
    }

    /// The home format a new file of this type gets when none is named.
    pub fn default_format(self) -> Format {
        match self {
            DocType::Document => Format::Docx,
            DocType::Presentation => Format::Pptx,
            DocType::Spreadsheet => Format::Xlsx,
        }
    }

    /// The type a text's front matter names (`type: …`), if it names one.
    pub fn of_text(text: &str) -> Option<DocType> {
        let mut lines = text.lines();
        if lines.next()?.trim_end() != "---" {
            return None;
        }
        lines
            .take_while(|l| l.trim_end() != "---")
            .find_map(|l| l.strip_prefix("type:"))
            .and_then(|v| DocType::parse(v.split('#').next().unwrap_or(v)))
    }
}

/// The home format (DESIGN.md §3, §2 rule 10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Docx,
    Hwpx,
    Pptx,
    Xlsx,
}

static DOCX: DocxEngine = DocxEngine;
static HWPX: HwpxEngine = HwpxEngine;
static PPTX: PptxEngine = PptxEngine;
static XLSX: XlsxEngine = XlsxEngine;

impl Format {
    pub const ALL: [Format; 4] = [Format::Docx, Format::Hwpx, Format::Pptx, Format::Xlsx];

    pub fn name(self) -> &'static str {
        match self {
            Format::Docx => "docx",
            Format::Hwpx => "hwpx",
            Format::Pptx => "pptx",
            Format::Xlsx => "xlsx",
        }
    }

    pub fn parse(s: &str) -> Option<Format> {
        Format::ALL.into_iter().find(|f| f.name() == s.trim().to_lowercase())
    }

    /// The format of a file name, by its extension; the macro-enabled
    /// variants (`.docm`, `.pptm`, `.xlsm`) open as their format, with the
    /// macros neutralised (§8).
    pub fn of_name(name: &str) -> Option<Format> {
        let ext = name.rsplit_once('.')?.1.to_lowercase();
        match ext.as_str() {
            "docx" | "docm" => Some(Format::Docx),
            "hwpx" => Some(Format::Hwpx),
            "pptx" | "pptm" => Some(Format::Pptx),
            "xlsx" | "xlsm" => Some(Format::Xlsx),
            _ => None,
        }
    }

    pub fn doc_type(self) -> DocType {
        match self {
            Format::Docx | Format::Hwpx => DocType::Document,
            Format::Pptx => DocType::Presentation,
            Format::Xlsx => DocType::Spreadsheet,
        }
    }

    pub fn engine(self) -> &'static dyn Engine {
        match self {
            Format::Docx => &DOCX,
            Format::Hwpx => &HWPX,
            Format::Pptx => &PPTX,
            Format::Xlsx => &XLSX,
        }
    }

    pub fn caps(self) -> Capabilities {
        self.engine().capabilities()
    }

    /// The text grammar edits and re-anchoring run on; `None` for a
    /// Spreadsheet, whose structure text the engine reconciles itself.
    pub fn text_model(self) -> Option<&'static dyn TextModel> {
        match self {
            Format::Docx | Format::Hwpx => Some(&DocumentModel),
            Format::Pptx => Some(&PptxModel),
            Format::Xlsx => None,
        }
    }

    /// A new revision's blocks with what its text could not show filled in
    /// from the remainder (docx: a paragraph that had no text keeps its
    /// formatting when it gets some, §5.2).
    pub fn complete(self, blocks: &mut [Block], rem: &mut Remainder) {
        if self == Format::Docx {
            DocxEngine::complete(blocks, rem);
        }
    }

    /// The canonical text of resolved blocks (Documents and Presentations).
    pub fn text_of(self, blocks: &[Block], rem: &Remainder, template: Option<&str>) -> String {
        match self {
            Format::Docx => DocxEngine::text_of(blocks, rem, template),
            Format::Hwpx => HwpxEngine::text_of(blocks, rem, template),
            Format::Pptx => PptxEngine::text_of(blocks, rem, template),
            Format::Xlsx => unreachable!("a spreadsheet's text is its structure"),
        }
    }

    /// The validator's errors for `text` against a file's remainder: its
    /// styles, layouts, slots and placeholders.
    pub fn check(self, text: &str, rem: &Remainder) -> Vec<Diagnostic> {
        match self.text_model() {
            Some(m) => m.resolve(text, rem, self.caps()).err().unwrap_or_default(),
            None => hanji_format::parse_spreadsheet(text, &XlsxEngine::names(rem)).err().unwrap_or_default(),
        }
    }
}

/// The validator's errors for `text` as a file of type `ty`, without a
/// file to check names against.
pub fn check_text(ty: DocType, text: &str) -> Vec<Diagnostic> {
    let names = Names::default();
    let r = match ty {
        DocType::Document => hanji_format::parse(text).map(|_| ()),
        DocType::Presentation => hanji_format::parse_presentation(text, &names).map(|_| ()),
        DocType::Spreadsheet => hanji_format::parse_spreadsheet(text, &names).map(|_| ()),
    };
    r.err().unwrap_or_default()
}
