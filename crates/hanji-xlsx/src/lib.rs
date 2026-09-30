//! The xlsx engine (SpreadsheetML, DESIGN.md §5.4). A workbook splits in two:
//! its structure is text (sheets, tables with their columns' types, number
//! formats and formulas, and placeholders for what the text does not model),
//! and its cells are read through row windows and written by range
//! operations. The cell grid stays in the remainder, in its home encoding
//! (each worksheet's rows, the shared strings), because a 100k-row sheet is
//! never one text blob (§2 rule 11); a window renders the rows it shows and
//! an operation rewrites the rows it touches. Every other part is copied
//! through byte for byte. No I/O: package bytes in, package bytes out.
//!
//! Neither umya-spreadsheet nor IronCalc is on the import/export path
//! (§7): both read the package into their own model and write it back from
//! it, and a model that does not hold charts, drawings or validations drops
//! them. IronCalc is the calculator only: after an operation it computes the
//! formulas whose inputs changed over the engine's own cells, and the engine
//! writes the cached values (`calc.rs`).

pub mod book;
pub mod calc;
pub mod drawing;
pub mod format;
pub mod model;
pub mod numfmt;
pub mod ops;
pub mod safety;
pub mod shift;
pub mod sst;
pub mod store;
pub mod structure;
pub mod styles;
pub mod value;
pub mod view;

pub use hanji_package::{package, xml};

use hanji_core::{Capabilities, Engine, EngineError, ImportOptions, ImportReport, Imported, Kind, Remainder, StyleSet};
use hanji_format::sheet::{parse_spreadsheet, serialize_spreadsheet, Spreadsheet, WindowOf};
use hanji_format::Names;

use crate::book::{Book, Shell};

pub struct XlsxEngine;

/// The result of range operations: the new revision's text and remainder,
/// and what happened to everything the remainder anchors.
#[derive(Clone, Debug)]
pub struct Applied {
    pub text: String,
    pub remainder: Remainder,
    pub report: ops::OpReport,
}

fn pkg(m: String) -> EngineError {
    EngineError::Package(m)
}

/// The book of a remainder (its range entries; placeholders are derived again).
pub(crate) fn book_of(rem: &Remainder) -> Result<Book, EngineError> {
    let shell: Shell = match rem.shell.first() {
        Some(s) => serde_json::from_str(s).map_err(|e| pkg(format!("the remainder's shell: {e}")))?,
        None => Shell::default(),
    };
    let entries = rem.entries.iter().filter(|e| e.kind == Kind::Range).cloned().collect();
    Book::load(rem.parts.clone(), entries, shell).map_err(pkg)
}

/// The remainder of a book's revision.
pub(crate) fn remainder_of(book: Book, mut next_id: u64) -> Remainder {
    let keeps = model::keep_entries(&book, &mut next_id);
    let (parts, mut entries, shell) = book.finish();
    entries.extend(keeps);
    Remainder {
        format: "xlsx".into(),
        namespaces: vec![],
        shell: vec![serde_json::to_string(&shell).expect("the shell serializes")],
        styles: StyleSet::default(),
        entries,
        parts,
        next_id,
    }
}

impl XlsxEngine {
    /// Import into a remainder, with the §8 report.
    pub fn split(package: &[u8], opts: &ImportOptions) -> Result<(Remainder, ImportReport), EngineError> {
        let parts = package::read(package).map_err(pkg)?;
        let mut book = Book::load(parts, vec![], Shell::default()).map_err(pkg)?;
        let mut report = ImportReport::default();
        if opts.neutralise {
            safety::neutralise(&mut book, &mut report).map_err(pkg)?;
        }
        safety::surface(&mut book, &mut report).map_err(pkg)?;
        let mut next_id = 1;
        for i in 0..book.sheets.len() {
            if book.sheets[i].kind == book::SheetKind::Work {
                model::extract_entries(&mut book, i, &mut next_id).map_err(pkg)?;
            }
        }
        Ok((remainder_of(book, next_id), report))
    }

    /// The structure of a remainder's revision.
    pub fn structure(rem: &Remainder, template: Option<&str>) -> Result<Spreadsheet, EngineError> {
        let mut book = book_of(rem)?;
        model::structure(&mut book, template).map_err(pkg)
    }

    /// The names the text may use: its placeholders.
    pub fn names(rem: &Remainder) -> Names {
        Names { keeps: Some(rem.keep_list()), formats: Some(vec!["xlsx".into()]), ..Default::default() }
    }

    pub fn text_of(rem: &Remainder, template: Option<&str>) -> Result<String, EngineError> {
        Ok(serialize_spreadsheet(&Self::structure(rem, template)?))
    }

    /// A row window (§5.4): a table's rows (`rows` `None`: every data row, at
    /// most `view::MAX_WINDOW`), or a range of a sheet.
    pub fn window(rem: &Remainder, of: &WindowOf) -> Result<String, EngineError> {
        let mut book = book_of(rem)?;
        view::window(&mut book, of).map_err(EngineError::Refused)
    }

    /// The structure with each table's first `rows` data rows as windows
    /// (round 4's read view A, for small workbooks and tests).
    pub fn view(rem: &Remainder, rows: u32) -> Result<String, EngineError> {
        let mut book = book_of(rem)?;
        view::full_view(&mut book, rows).map_err(EngineError::Refused)
    }

    /// Compute the formulas that have no cached value (a file written without
    /// them, as openpyxl writes one), and those that read them, and write
    /// their cached values. Cached values the file has are kept.
    pub fn recalculate(rem: &Remainder) -> Result<(Remainder, calc::Recalc), EngineError> {
        let mut book = book_of(rem)?;
        let ch = calc::uncached_formulas(&mut book).map_err(pkg)?;
        let rc = calc::recompute(&mut book, &ch).map_err(pkg)?;
        Ok((remainder_of(book, rem.next_id), rc))
    }

    /// Apply structure edits in `text` and then the range operations in `ops`
    /// (a JSON list, §5.4), all or nothing.
    pub fn apply(text: &str, rem: &Remainder, ops_json: &str) -> Result<Applied, EngineError> {
        ops::apply(text, rem, ops_json)
    }
}

impl Engine for XlsxEngine {
    fn format(&self) -> &'static str {
        "xlsx"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }

    fn import(&self, package: &[u8], opts: &ImportOptions) -> Result<Imported, EngineError> {
        let (remainder, report) = Self::split(package, opts)?;
        let text = Self::text_of(&remainder, opts.template.as_deref())?;
        Ok(Imported { text, remainder, report })
    }

    fn export(&self, text: &str, rem: &Remainder) -> Result<Vec<u8>, EngineError> {
        let parsed = parse_spreadsheet(text, &Self::names(rem)).map_err(EngineError::Invalid)?;
        let current = Self::structure(rem, parsed.front.template.as_deref())?;
        let rem = if structure::same(&parsed, &current) { rem.clone() } else { ops::apply(text, rem, "[]")?.remainder };
        let mut book = book_of(&rem)?;
        for i in 0..book.sheets.len() {
            model::restore_entries(&mut book, i).map_err(pkg)?;
        }
        calc::mark_recalc(&mut book);
        let (parts, _, _) = book.finish();
        package::write(&parts).map_err(pkg)
    }
}
