//! hanji's documents and revisions, and the operations of the `hanji` CLI
//! (DESIGN.md §4): open, new, read, edit, write, range operations,
//! validate, export, re-import, history and diff.
//!
//! A document is stored as what §4 says is the truth: for each revision its
//! model text and its remainder (serialized as the core serializes it), and
//! once its home format, template and revision history. The package is never
//! stored; export makes it from a revision. Every change makes a revision:
//! import, an exact-span edit, a whole-file rewrite (design C), range
//! operations, and a re-import of a file a person polished (rule 7).
//!
//! This layer adds no document semantics: edits and re-anchoring are the
//! core's, validation the format's, import, export, §8 neutralisation and
//! the surface list the engines'. It checks revisions, finds `old` in the
//! text, rebases an edit over a person's re-import as a text merge, and
//! writes errors for the model.
//!
//! No I/O but through [`Storage`]; the path operations (`open`, `export`,
//! …) are native only.

pub mod blank;
pub mod error;
pub mod formats;
pub mod merge;
pub mod storage;
pub mod view;

use hanji_core::edit::{edit_in, reanchor_span_in, rewrite_in, Reanchored};
use hanji_core::remainder::fnv1a;
use hanji_core::{ImportOptions, Remainder, Report};
use hanji_format::sheet::WindowOf;
use hanji_xlsx::XlsxEngine;
use serde::{Deserialize, Serialize};

pub use error::{Code, Conflict, Detail, Diag, Error, Item, Loss, Result};
pub use formats::{DocType, Format};
#[cfg(not(target_family = "wasm"))]
pub use storage::FsStorage;
pub use storage::{MemStorage, Storage};
pub use view::{OutlineEntry, Window};

use error::{diags, items};
use merge::{line_of, Hunk, Merged};

/// The date tracked changes carry: fixed, so the same revision exports the same bytes (§8).
const TRACKED_DATE: &str = "2026-01-01T00:00:00Z";

/// How to work with hanji and the format, written for a model (`hanji
/// guide`).
pub const GUIDE: &str = include_str!("guide.md");

/// What made a revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RevOp {
    Open,
    New,
    Edit,
    Write,
    Ops,
    Reimport,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revision {
    pub id: u32,
    pub parent: Option<u32>,
    pub op: RevOp,
    pub summary: String,
}

/// A document: its home format, template and revisions. The text and
/// remainder of each revision are stored beside it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocRecord {
    pub id: String,
    pub doc_type: DocType,
    pub format: Format,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// The file it was opened from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub head: u32,
    pub revisions: Vec<Revision>,
}

/// One exact-span edit: the one occurrence of `old` becomes `new`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEdit {
    pub old: String,
    pub new: String,
}

/// §8 on import: what was neutralised, and what to surface before export.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    pub neutralised: Vec<Item>,
    pub surfaced: Vec<Item>,
}

impl From<&hanji_core::ImportReport> for ImportReport {
    fn from(r: &hanji_core::ImportReport) -> ImportReport {
        ImportReport { neutralised: items(&r.neutralised), surfaced: items(&r.surface) }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Opened {
    pub doc_id: String,
    pub revision: u32,
    pub doc_type: DocType,
    pub format: Format,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    pub lines: usize,
    pub report: ImportReport,
}

#[derive(Clone, Debug, Serialize)]
pub struct Read {
    pub doc_id: String,
    pub revision: u32,
    /// The current revision (edits go against it).
    pub head: u32,
    pub doc_type: DocType,
    pub format: Format,
    /// The lines `text` holds, 1-based and inclusive, of `total_lines`.
    pub first_line: usize,
    pub last_line: usize,
    pub total_lines: usize,
    /// `text` is part of the revision.
    pub partial: bool,
    /// The revision's text, or the part asked for, exactly as stored.
    pub text: String,
    /// Headings, slides, or sheets and tables, when `text` is part of the file.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub outline: Vec<OutlineEntry>,
    /// A Spreadsheet's row window (read-only: write cells with range operations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Where to read on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

/// A range entry of a Spreadsheet that moved or went.
#[derive(Clone, Debug, Serialize)]
pub struct Moved {
    pub tag: String,
    pub sheet: String,
    pub before: String,
    pub after: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Recalc {
    pub dirty: usize,
    pub written: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub left: Vec<String>,
}

/// A new revision (or none, when the text did not change).
#[derive(Clone, Debug, Serialize)]
pub struct Changed {
    pub doc_id: String,
    pub revision: u32,
    pub parent: u32,
    /// Nothing changed; no revision was made.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub unchanged: bool,
    /// The stored text is the canonical form of what was written, which
    /// differs from it: read again before the next edit near the change.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub canonicalized: bool,
    /// Re-imported revisions (a person's edits) the change was merged over.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rebased_over: Vec<u32>,
    /// Remainder entries kept (Documents, Presentations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placed: Option<usize>,
    /// Entries removed with the text or block they belonged to.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<Loss>,
    /// Range operations applied (Spreadsheets; structure edits first).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub moved: Vec<Moved>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Item>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recalc: Option<Recalc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Validated {
    pub valid: bool,
    pub doc_type: DocType,
    /// Names were checked against this document's styles, layouts and placeholders.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_id: Option<String>,
    pub diagnostics: Vec<Diag>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportOptions {
    /// Export although the §8 surface list is not empty (after a person saw it).
    pub acknowledge_surfaced: bool,
    /// Write the model's edits as tracked changes (docx, §10.2).
    pub tracked_changes: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Exported {
    pub doc_id: String,
    pub revision: u32,
    pub format: Format,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub bytes: usize,
    /// Identifies the export: the same revision gives the same bytes (§8).
    pub digest: String,
    /// What leaves with the file that nobody may have reviewed (§8).
    pub surfaced: Vec<Item>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Reimported {
    pub doc_id: String,
    pub revision: u32,
    pub parent: u32,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub unchanged: bool,
    pub report: ImportReport,
    /// The person's changes to the text, as a unified diff.
    pub diff: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct History {
    pub doc_id: String,
    pub doc_type: DocType,
    pub format: Format,
    pub head: u32,
    pub revisions: Vec<Revision>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Diff {
    pub doc_id: String,
    pub from: u32,
    pub to: u32,
    pub diff: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocSummary {
    pub doc_id: String,
    pub doc_type: DocType,
    pub format: Format,
    pub head: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// A new revision's content, before it is stored.
struct Next {
    text: String,
    rem: Remainder,
    changed: Changed,
}

/// The documents in a [`Storage`], and the operations on them.
pub struct Workspace<S: Storage> {
    storage: S,
}

fn doc_key(id: &str) -> String {
    format!("docs/{id}/doc.json")
}

fn text_key(id: &str, rev: u32) -> String {
    format!("docs/{id}/r{rev}.txt")
}

fn rem_key(id: &str, rev: u32) -> String {
    format!("docs/{id}/r{rev}.remainder.json")
}

/// A document id from a file name: letters and digits (any script), `-`, `_`.
fn slug(name: &str) -> String {
    let stem = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let stem = stem.rsplit_once('.').map_or(stem, |s| s.0);
    let mut s = String::new();
    for c in stem.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() || c == '_' {
            s.push(c);
        } else if !s.ends_with('-') {
            s.push('-');
        }
    }
    let s: String = s.trim_matches('-').chars().take(40).collect();
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() {
        "doc".into()
    } else {
        s
    }
}

/// The base document id derived from a filename, before store collision suffixes.
/// Stateless preview adapters use the same safe filename policy as `open_bytes`.
pub fn id_from_name(name: &str) -> String {
    slug(name)
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 200 && id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Every occurrence of `old` in `text` (overlapping ones included), as the core counts them.
fn occurrences(text: &str, old: &str) -> Vec<usize> {
    let mut hits = vec![];
    let mut from = 0;
    while let Some(k) = text[from..].find(old) {
        hits.push(from + k);
        from += k + text[from + k..].chars().next().map_or(1, char::len_utf8);
    }
    hits
}

/// Where the one occurrence of `old` starts, or why there is not one.
fn locate(text: &str, old: &str, rev: u32) -> Result<usize> {
    if old.is_empty() {
        return Err(Error::bad(
            "old is empty: include text from the place to edit (to add a paragraph, include the line before it).",
        ));
    }
    let hits = occurrences(text, old);
    match hits.as_slice() {
        [one] => Ok(*one),
        [] => {
            let mut msg =
                format!("the old text is not in revision {rev}; copy it exactly from a read of that revision.");
            let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
            let first = old.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
            let at: Vec<usize> = if first.is_empty() {
                vec![]
            } else {
                occurrences(text, first).iter().map(|&k| line_of(text, k)).collect()
            };
            if let Some(near) = hanji_format::chars::near_miss(text, old) {
                // Look-alike characters: name them where it would match.
                msg.push(' ');
                msg.push_str(&near.message);
            } else if !old.trim().is_empty() && squash(text).contains(&squash(old)) {
                msg.push_str(" It matches if spaces and line breaks are ignored: copy them exactly as the revision has them (no padding, one paragraph per line).");
            } else if !at.is_empty() {
                let shown: Vec<String> = at.iter().take(10).map(|l| l.to_string()).collect();
                msg.push_str(&format!(
                    " Its first line {first:?} is at line {}; the lines after it differ.",
                    shown.join(", ")
                ));
            }
            let mut e = Error::new(Code::NoMatch, msg);
            e.detail.matches = Some(0);
            e.detail.lines = at;
            Err(e)
        }
        many => {
            let ls: Vec<usize> = many.iter().map(|&k| line_of(text, k)).collect();
            let shown: Vec<String> = ls.iter().take(20).map(|l| l.to_string()).collect();
            let mut e = Error::new(
                Code::AmbiguousMatch,
                format!(
                    "the old text occurs {} times (lines {}); include more of the text around it so it occurs once.",
                    many.len(),
                    shown.join(", ")
                ),
            );
            e.detail.matches = Some(many.len());
            e.detail.lines = ls;
            Err(e)
        }
    }
}

fn losses(r: &Report) -> Vec<Loss> {
    r.removed.iter().map(|(_, k, why)| Loss::of(*k, why)).collect()
}

fn engine_err(e: hanji_core::EngineError) -> Error {
    match e {
        hanji_core::EngineError::Refused(m) if m.starts_with("the operations are not valid") => {
            Error::new(Code::Invalid, m)
        }
        e => e.into(),
    }
}

impl<S: Storage> Workspace<S> {
    pub fn new(storage: S) -> Workspace<S> {
        Workspace { storage }
    }

    // ------------------------------------------------------------ storage

    fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.storage.get(key).map_err(Error::io)
    }

    fn put(&mut self, key: &str, data: &[u8]) -> Result<()> {
        self.storage.put(key, data).map_err(Error::io)
    }

    /// A document's record.
    pub fn doc(&self, id: &str) -> Result<DocRecord> {
        let found = if valid_id(id) { self.get(&doc_key(id))? } else { None };
        let Some(data) = found else {
            let ids = self.ids()?;
            let ids: Vec<&str> = ids.iter().map(String::as_str).take(20).collect();
            return Err(Error::not_found(format!(
                "there is no document {id:?}. Documents: {}.",
                if ids.is_empty() { "none (open or create one first)".into() } else { ids.join(", ") }
            )));
        };
        serde_json::from_slice(&data).map_err(|e| Error::io(format!("the record of {id} is damaged: {e}")))
    }

    fn check_rev(doc: &DocRecord, rev: u32) -> Result<()> {
        if doc.revisions.iter().any(|r| r.id == rev) {
            Ok(())
        } else {
            let mut e = Error::not_found(format!(
                "document {} has no revision {rev}; its revisions are 1–{}, the current one {}.",
                doc.id,
                doc.revisions.len(),
                doc.head
            ));
            e.detail.head = Some(doc.head);
            Err(e)
        }
    }

    /// The text of a revision.
    pub fn text(&self, doc: &DocRecord, rev: u32) -> Result<String> {
        Self::check_rev(doc, rev)?;
        let damaged = || Error::io(format!("revision {rev} of {} has no UTF-8 text", doc.id));
        let text = self.get(&text_key(&doc.id, rev))?.ok_or_else(damaged)?;
        String::from_utf8(text).map_err(|_| damaged())
    }

    /// The text and remainder of a revision.
    pub fn revision(&self, doc: &DocRecord, rev: u32) -> Result<(String, Remainder)> {
        let text = self.text(doc, rev)?;
        let damaged = || Error::io(format!("revision {rev} of {} has no remainder", doc.id));
        let rem = self.get(&rem_key(&doc.id, rev))?.ok_or_else(damaged)?;
        let rem = Remainder::from_json(std::str::from_utf8(&rem).map_err(|_| damaged())?)
            .map_err(|e| Error::io(format!("revision {rev} of {}: {e}", doc.id)))?;
        Ok((text, rem))
    }

    fn commit(&mut self, doc: &mut DocRecord, op: RevOp, summary: String, text: &str, rem: &Remainder) -> Result<u32> {
        let id = doc.revisions.iter().map(|r| r.id).max().unwrap_or(0) + 1;
        self.put(&text_key(&doc.id, id), text.as_bytes())?;
        self.put(&rem_key(&doc.id, id), rem.to_json().as_bytes())?;
        let parent = (!doc.revisions.is_empty()).then_some(doc.head);
        doc.revisions.push(Revision { id, parent, op, summary });
        doc.head = id;
        // The record last: a revision exists once the record names it.
        let rec = serde_json::to_vec_pretty(doc).expect("a record serializes");
        self.put(&doc_key(&doc.id), &rec)?;
        Ok(id)
    }

    fn fresh_id(&self, name: &str) -> Result<String> {
        let base = slug(name);
        let mut id = base.clone();
        let mut k = 2;
        while self.get(&doc_key(&id))?.is_some() {
            id = format!("{base}-{k}");
            k += 1;
        }
        Ok(id)
    }

    /// The revisions after `rev` up to the head, oldest first. Refused as
    /// stale unless every one of them is a re-import (rule 7: a person's
    /// edits, which a model edit is rebased over).
    fn after(doc: &DocRecord, rev: u32) -> Result<Vec<u32>> {
        Self::check_rev(doc, rev)?;
        let mut chain = vec![];
        let mut cur = doc.head;
        while cur != rev {
            let r = doc.revisions.iter().find(|r| r.id == cur).expect("the chain is stored");
            chain.push(r);
            cur = match r.parent {
                Some(p) => p,
                None => break,
            };
        }
        chain.reverse();
        if let Some(r) = chain.iter().find(|r| r.op != RevOp::Reimport) {
            let mut e = Error::new(
                Code::StaleRevision,
                format!(
                    "revision {rev} is not the current revision of {}: revision {} ({}) changed it since, and the current one is {}. Read revision {} and make the edit against it.",
                    doc.id,
                    r.id,
                    r.summary,
                    doc.head,
                    doc.head
                ),
            );
            e.detail.head = Some(doc.head);
            return Err(e);
        }
        Ok(chain.iter().map(|r| r.id).collect())
    }

    // ------------------------------------------------------------ operations

    /// The documents in the store.
    /// The ids of the documents in the store.
    fn ids(&self) -> Result<Vec<String>> {
        let keys = self.storage.keys("docs/").map_err(Error::io)?;
        Ok(keys
            .iter()
            .filter_map(|k| k.strip_prefix("docs/")?.strip_suffix("/doc.json"))
            .filter(|id| valid_id(id))
            .map(str::to_string)
            .collect())
    }

    pub fn list(&self) -> Result<Vec<DocSummary>> {
        let mut out = vec![];
        for id in self.ids()? {
            let d = self.doc(&id)?;
            out.push(DocSummary {
                doc_id: d.id,
                doc_type: d.doc_type,
                format: d.format,
                head: d.head,
                source: d.source,
            });
        }
        Ok(out)
    }

    fn import(
        &mut self,
        id_from: &str,
        format: Format,
        bytes: &[u8],
        template: Option<String>,
        source: Option<String>,
        op: RevOp,
    ) -> Result<Opened> {
        let opts = ImportOptions { template: template.clone(), ..Default::default() };
        let imp = format.engine().import(bytes, &opts)?;
        let id = self.fresh_id(id_from)?;
        let mut doc = DocRecord {
            id: id.clone(),
            doc_type: format.doc_type(),
            format,
            template: template.clone(),
            source,
            head: 0,
            revisions: vec![],
        };
        let summary = match op {
            RevOp::New => format!("new {} ({})", format.doc_type().name(), template.as_deref().unwrap_or("blank")),
            _ => format!("opened {id_from}"),
        };
        let revision = self.commit(&mut doc, op, summary, &imp.text, &imp.remainder)?;
        Ok(Opened {
            doc_id: id,
            revision,
            doc_type: format.doc_type(),
            format,
            template,
            lines: imp.text.lines().count(),
            report: (&imp.report).into(),
        })
    }

    /// Import a package (§8 neutralisation runs); `name` gives its format
    /// and the document id.
    pub fn open_bytes(&mut self, name: &str, bytes: &[u8], source: Option<String>) -> Result<Opened> {
        let format = Format::of_name(name).ok_or_else(|| {
            Error::bad(format!("cannot tell the format of {name:?}: hanji opens .docx, .hwpx, .pptx and .xlsx files."))
        })?;
        self.import(name, format, bytes, None, source, RevOp::Open)
    }

    /// A new file of `ty` (rule 3): the blank package of its format, or a
    /// template package, imported. `template` is `(reference, package)`;
    /// the reference is written in the front matter.
    pub fn create(&mut self, ty: DocType, format: Option<Format>, template: Option<(&str, &[u8])>) -> Result<Opened> {
        let format = format.unwrap_or_else(|| ty.default_format());
        if format.doc_type() != ty {
            let allowed: Vec<&str> = Format::ALL.iter().filter(|f| f.doc_type() == ty).map(|f| f.name()).collect();
            return Err(Error::bad(format!(
                "a {} is written as {}, not {}.",
                ty.name(),
                allowed.join(" or "),
                format.name()
            )));
        }
        let (bytes, reference) = match template {
            Some((r, b)) => (b.to_vec(), Some(r.to_string())),
            None => (blank::package(format), None),
        };
        let name = reference.clone().unwrap_or_else(|| ty.name().to_string());
        self.import(&name, format, &bytes, reference, None, RevOp::New)
    }

    /// A revision's text (the current one by default), or the part `w` asks
    /// for; for a Spreadsheet also a row window.
    pub fn read(&self, id: &str, revision: Option<u32>, w: &Window) -> Result<Read> {
        let doc = self.doc(id)?;
        let rev = revision.unwrap_or(doc.head);
        let (text, rem) = self.revision(&doc, rev)?;
        let ty = doc.doc_type;
        if w.is_cells() && ty != DocType::Spreadsheet {
            return Err(Error::bad(
                "table, rows, sheet and range are a Spreadsheet's; read this file by lines, section or slides.",
            ));
        }
        let ls: Vec<&str> = text.split_inclusive('\n').collect();
        let total = ls.len();
        let (a, b) = view::part(ty, &text, w)?.unwrap_or((0, total));
        let end = view::clip(&ls, a, b, view::MAX_READ_BYTES);
        let part = ls[a..end].concat();
        let partial = a > 0 || end < total;
        let next =
            (end < b).then(|| format!("lines {}:{} (this part stops at {} bytes)", end + 1, b, view::MAX_READ_BYTES));
        let data = if ty == DocType::Spreadsheet { self.cells(&text, &rem, w)? } else { None };
        Ok(Read {
            doc_id: doc.id,
            revision: rev,
            head: doc.head,
            doc_type: ty,
            format: doc.format,
            first_line: a + 1,
            last_line: end,
            total_lines: total,
            partial,
            text: part,
            outline: if partial { view::outline(ty, &text) } else { vec![] },
            data,
            next,
        })
    }

    /// A Spreadsheet's row window: the one asked for, else the first rows
    /// of the first table (or of the first sheet's used range).
    fn cells(&self, text: &str, rem: &Remainder, w: &Window) -> Result<Option<String>> {
        use hanji_core::cells::{CellRange, CellRef};
        let win = |of: &WindowOf| XlsxEngine::window(rem, of).map_err(Error::from);
        if let Some(t) = &w.table {
            if w.sheet.is_some() || w.range.is_some() {
                return Err(Error::bad("ask for a table (with rows) or a sheet with a range, not both."));
            }
            let rows = w.rows.as_deref().map(|r| view::span(r, "rows")).transpose()?.map(|(a, b)| (a as u32, b as u32));
            return win(&WindowOf::Table { name: t.clone(), rows }).map(Some);
        }
        if w.rows.is_some() {
            return Err(Error::bad("rows are a table's: name the table too."));
        }
        if let Some(r) = &w.range {
            let (sheet, range) = match (&w.sheet, hanji_format::ops::split_range(r)) {
                (Some(s), _) => (s.clone(), r.clone()),
                (None, Some((s, r))) if !s.is_empty() => (s, r),
                _ => {
                    return Err(Error::bad("name the sheet of the range: sheet and range, or range \"Sheet1!A1:F50\"."))
                }
            };
            return win(&WindowOf::Range { sheet, range }).map(Some);
        }
        if w.sheet.is_some() {
            return Err(Error::bad("name the range of the sheet to read, such as \"A1:F50\"."));
        }
        let s = hanji_format::parse_spreadsheet(text, &XlsxEngine::names(rem)).map_err(|d| Error::invalid(&d))?;
        if let Some((_, t)) = s.tables().next() {
            let rows = CellRange::parse(&t.range).and_then(|r| {
                let first = r.first.row + 1;
                (r.last.row >= first + view::DEFAULT_ROWS).then(|| (first, first + view::DEFAULT_ROWS - 1))
            });
            let of = WindowOf::Table { name: t.name.clone(), rows };
            return win(&of).or_else(|_| win(&WindowOf::Table { name: t.name.clone(), rows: None })).map(Some);
        }
        let Some(sh) = s.sheets.iter().find(|sh| sh.range.is_some()) else { return Ok(None) };
        let r = CellRange::parse(sh.range.as_deref().unwrap())
            .ok_or_else(|| Error::bad("the sheet's range is not a range"))?;
        let last = CellRef::new(r.last.col.min(r.first.col + 25), r.last.row.min(r.first.row + view::DEFAULT_ROWS - 1));
        win(&WindowOf::Range { sheet: sh.name.clone(), range: CellRange::new(r.first, last).to_string() }).map(Some)
    }

    /// Check a text (§5): its own grammar, and against a document's names
    /// when `id` is given. The type comes from `ty`, else the document,
    /// else the front matter.
    pub fn validate(&self, text: &str, ty: Option<DocType>, id: Option<&str>) -> Result<Validated> {
        let doc = id.map(|i| self.doc(i)).transpose()?;
        let ty = ty.or(doc.as_ref().map(|d| d.doc_type)).or_else(|| DocType::of_text(text)).ok_or_else(|| {
            Error::bad("name the type (document, presentation or spreadsheet): the text has no front matter saying it.")
        })?;
        let ds = match &doc {
            Some(d) if d.doc_type != ty => {
                return Err(Error::bad(format!("document {} is a {}, not a {}.", d.id, d.doc_type.name(), ty.name())))
            }
            Some(d) => d.format.check(text, &self.revision(d, d.head)?.1),
            None => formats::check_text(ty, text),
        };
        Ok(Validated { valid: ds.is_empty(), doc_type: ty, doc_id: doc.map(|d| d.id), diagnostics: diags(&ds) })
    }

    /// Exact-span edits, applied in order against revision `rev` (which
    /// must be the current one, or older by re-imports only).
    pub fn edit(&mut self, id: &str, rev: u32, edits: &[TextEdit]) -> Result<Changed> {
        if edits.is_empty() {
            return Err(Error::bad("no edits: give old and new (or a list of edits)."));
        }
        let mut doc = self.doc(id)?;
        let over = Self::after(&doc, rev)?;
        let (text, rem) = self.revision(&doc, doc.head)?;
        let next = if over.is_empty() {
            match doc.format.text_model() {
                Some(m) => {
                    let (ra, removed) = chain(&text, &rem, edits.len(), |k, t, r| {
                        let (e, at) = (&edits[k], |x: Error| x.at_edit(k, edits.len()));
                        locate(t, &e.old, rev).map_err(at)?;
                        edit_in(m, r, t, &e.old, &e.new, doc.format.caps()).map_err(|x| at(x.into()))
                    })?;
                    self.finish(&doc, &text, ra.expect("at least one edit"), removed)?
                }
                None => {
                    let mut t = text.clone();
                    for (k, e) in edits.iter().enumerate() {
                        let s = locate(&t, &e.old, rev).map_err(|x| x.at_edit(k, edits.len()))?;
                        t.replace_range(s..s + e.old.len(), &e.new);
                    }
                    self.structure(&doc, &text, &t, &rem)?
                }
            }
        } else {
            let base = self.text(&doc, rev)?;
            let mut ours = base.clone();
            for (k, e) in edits.iter().enumerate() {
                let s = locate(&ours, &e.old, rev).map_err(|x| x.at_edit(k, edits.len()))?;
                ours.replace_range(s..s + e.old.len(), &e.new);
            }
            self.rebase(&doc, rev, &over, &base, &ours, &text, &rem)?
        };
        self.store(&mut doc, RevOp::Edit, edit_summary(edits), next)
    }

    /// A whole-file rewrite (design C): `text` replaces revision `rev`.
    pub fn write(&mut self, id: &str, rev: u32, new_text: &str) -> Result<Changed> {
        let mut doc = self.doc(id)?;
        let over = Self::after(&doc, rev)?;
        let (text, rem) = self.revision(&doc, doc.head)?;
        let next = if over.is_empty() {
            match doc.format.text_model() {
                Some(m) => {
                    let ra = rewrite_in(m, &rem, &text, new_text, doc.format.caps())?;
                    let removed = losses(&ra.report);
                    self.finish(&doc, &text, ra, removed)?
                }
                None => self.structure(&doc, &text, new_text, &rem)?,
            }
        } else {
            let base = self.text(&doc, rev)?;
            self.rebase(&doc, rev, &over, &base, new_text, &text, &rem)?
        };
        let summary = format!("rewrote the text ({} lines)", new_text.lines().count());
        self.store(&mut doc, RevOp::Write, summary, next)
    }

    /// Range operations on a Spreadsheet (§5.4): a JSON list, applied in
    /// order, all or nothing.
    pub fn ops(&mut self, id: &str, rev: u32, ops_json: &str) -> Result<Changed> {
        let mut doc = self.doc(id)?;
        if doc.format != Format::Xlsx {
            return Err(Error::bad(format!(
                "range operations are a Spreadsheet's; {} is a {}: change it with edit or write.",
                doc.id,
                doc.doc_type.name()
            )));
        }
        if !Self::after(&doc, rev)?.is_empty() {
            let mut e = Error::new(
                Code::StaleRevision,
                format!(
                    "revision {rev} is not the current revision of {}: a person's edits were re-imported since (revision {}). Range operations are not rebased; read revision {} and send them again.",
                    doc.id, doc.head, doc.head
                ),
            );
            e.detail.head = Some(doc.head);
            return Err(e);
        }
        let (text, rem) = self.revision(&doc, doc.head)?;
        let a = XlsxEngine::apply(&text, &rem, ops_json).map_err(engine_err)?;
        let n = a.report.applied;
        let next = self.applied(&doc, &text, a, &text)?;
        self.store(&mut doc, RevOp::Ops, format!("{n} range operation{}", if n == 1 { "" } else { "s" }), next)
    }

    /// Export a revision (the current one by default) to package bytes.
    /// Refused while the §8 surface list is not empty, unless acknowledged.
    pub fn export_bytes(&self, id: &str, revision: Option<u32>, opts: &ExportOptions) -> Result<(Exported, Vec<u8>)> {
        let doc = self.doc(id)?;
        let rev = revision.unwrap_or(doc.head);
        let (text, rem) = self.revision(&doc, rev)?;
        let engine = doc.format.engine();
        let bytes = if opts.tracked_changes { self.tracked_bytes(&doc, rev)? } else { engine.export(&text, &rem)? };
        // What leaves with the file is what the exported package holds.
        let back = engine.import(&bytes, &ImportOptions::default())?;
        let surfaced = items(&back.report.surface);
        if !surfaced.is_empty() && !opts.acknowledge_surfaced {
            let list: Vec<String> =
                surfaced.iter().map(|s| format!("- {} at {}: {}", s.kind, s.location, s.detail)).collect();
            let mut e = Error::new(
                Code::SurfacedNotAcknowledged,
                format!(
                    "the file would carry content nobody may have reviewed (§8):\n{}\nShow this list to the person; export again with --acknowledge-surfaced once they have seen it, or remove the content first.",
                    list.join("\n")
                ),
            );
            e.detail.surfaced = surfaced;
            return Err(e);
        }
        let out = Exported {
            doc_id: doc.id,
            revision: rev,
            format: doc.format,
            path: None,
            bytes: bytes.len(),
            digest: format!("fnv1a64:{:016x}", fnv1a(bytes.iter().copied())),
            surfaced,
        };
        Ok((out, bytes))
    }

    /// A docx export with the model's edits since the file was last
    /// imported (opened or re-imported) as tracked changes (§10.2). The
    /// store keeps each revision's text, so every revision after that import
    /// is replayed as exact edits (each run of changed lines one edit).
    fn tracked_bytes(&self, doc: &DocRecord, rev: u32) -> Result<Vec<u8>> {
        if doc.format != Format::Docx {
            return Err(Error::new(
                Code::Unsupported,
                format!(
                    "tracked changes are written to docx only (§10.2); {} is {}: export without them.",
                    doc.id,
                    doc.format.name()
                ),
            ));
        }
        // The revisions from the last import to `rev`.
        let mut path = vec![];
        let mut at = Some(rev);
        while let Some(r) = at {
            let revision = doc
                .revisions
                .iter()
                .find(|x| x.id == r)
                .ok_or_else(|| Error::bad(format!("{} has no revision {r}", doc.id)))?;
            path.push(r);
            if matches!(revision.op, RevOp::Open | RevOp::New | RevOp::Reimport) {
                break;
            }
            at = revision.parent;
        }
        path.reverse();
        let (text, rem) = self.revision(doc, path[0])?;
        let mut h = hanji_docx::History::new(&text, &rem)?;
        for r in &path[1..] {
            h.push_text(&self.text(doc, *r)?)?;
        }
        let reviewer = hanji_docx::Reviewer { author: "hanji (model edit)".into(), date: TRACKED_DATE.into() };
        let opts = hanji_docx::ExportOptions { tracked_changes: Some(reviewer) };
        Ok(hanji_docx::DocxEngine.export_with(h.text(), h.remainder(), &opts, Some(&h))?)
    }

    /// A person's edits come back in (rule 7): the file, polished in the
    /// native application, becomes a new revision.
    pub fn reimport_bytes(&mut self, id: &str, name: &str, bytes: &[u8]) -> Result<Reimported> {
        let mut doc = self.doc(id)?;
        if let Some(f) = Format::of_name(name).filter(|f| *f != doc.format) {
            return Err(Error::bad(format!(
                "{} is a {} document; {name:?} is {}.",
                doc.id,
                doc.format.name(),
                f.name()
            )));
        }
        let opts = ImportOptions { template: doc.template.clone(), ..Default::default() };
        let imp = doc.format.engine().import(bytes, &opts)?;
        let parent = doc.head;
        let (text, rem) = self.revision(&doc, parent)?;
        let diff = merge::unified(&text, &imp.text, &format!("revision {parent}"), name);
        let report = (&imp.report).into();
        // Nothing changed when the two revisions make the same package.
        let engine = doc.format.engine();
        let same = |a: &str, ra: &Remainder, b: &str, rb: &Remainder| match (engine.export(a, ra), engine.export(b, rb))
        {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        };
        if text == imp.text && same(&text, &rem, &imp.text, &imp.remainder) {
            return Ok(Reimported { doc_id: doc.id, revision: parent, parent, unchanged: true, report, diff });
        }
        let summary = format!("re-imported {name}");
        let revision = self.commit(&mut doc, RevOp::Reimport, summary, &imp.text, &imp.remainder)?;
        Ok(Reimported { doc_id: doc.id, revision, parent, unchanged: false, report, diff })
    }

    pub fn history(&self, id: &str) -> Result<History> {
        let d = self.doc(id)?;
        Ok(History { doc_id: d.id, doc_type: d.doc_type, format: d.format, head: d.head, revisions: d.revisions })
    }

    /// The text diff of revision `from` → `to` (unified, by lines).
    pub fn diff(&self, id: &str, from: u32, to: u32) -> Result<Diff> {
        let doc = self.doc(id)?;
        let (a, b) = (self.text(&doc, from)?, self.text(&doc, to)?);
        let diff = merge::unified(&a, &b, &format!("revision {from}"), &format!("revision {to}"));
        Ok(Diff { doc_id: doc.id, from, to, diff })
    }

    // ------------------------------------------------------------ helpers

    /// A re-anchored Document or Presentation revision: exported once so a
    /// refusal comes now, not at export, and stored in canonical form.
    fn finish(&self, doc: &DocRecord, before: &str, mut ra: Reanchored, removed: Vec<Loss>) -> Result<Next> {
        let f = doc.format;
        f.complete(&mut ra.new, &mut ra.remainder);
        f.engine().export(&ra.text, &ra.remainder)?;
        let canon = f.text_of(&ra.new, &ra.remainder, doc.template.as_deref());
        let same_blocks =
            |t: &str| f.text_model().unwrap().resolve(t, &ra.remainder, f.caps()).is_ok_and(|(b, _)| b == ra.new);
        let (text, canonicalized) =
            if canon != ra.text && same_blocks(&canon) { (canon, true) } else { (ra.text.clone(), false) };
        let mut changed = self.changed(doc, before == text);
        changed.canonicalized = canonicalized;
        changed.placed = Some(ra.report.placed);
        changed.removed = removed;
        Ok(Next { text, rem: ra.remainder, changed })
    }

    /// A Spreadsheet's structure text edited: the engine reconciles it.
    fn structure(&self, doc: &DocRecord, before: &str, new_text: &str, rem: &Remainder) -> Result<Next> {
        let a = XlsxEngine::apply(new_text, rem, "[]").map_err(engine_err)?;
        self.applied(doc, before, a, new_text)
    }

    fn applied(&self, doc: &DocRecord, before: &str, a: hanji_xlsx::Applied, written: &str) -> Result<Next> {
        let mut changed = self.changed(doc, false);
        changed.canonicalized = a.text != written;
        changed.applied = Some(a.report.applied);
        changed.moved = a
            .report
            .entries
            .iter()
            .filter(|e| e.after.as_deref() != Some(e.before.as_str()))
            .map(|e| Moved {
                tag: e.tag.clone(),
                sheet: e.sheet.clone(),
                before: e.before.clone(),
                after: e.after.clone(),
            })
            .collect();
        changed.notices = items(&a.report.notices);
        let rc = &a.report.recalc;
        changed.recalc = (rc.dirty > 0).then(|| Recalc { dirty: rc.dirty, written: rc.written, left: rc.left.clone() });
        changed.unchanged = a.text == before && a.report.applied == 0;
        Ok(Next { text: a.text, rem: a.remainder, changed })
    }

    fn changed(&self, doc: &DocRecord, unchanged: bool) -> Changed {
        Changed {
            doc_id: doc.id.clone(),
            revision: doc.head,
            parent: doc.head,
            unchanged,
            canonicalized: false,
            rebased_over: vec![],
            placed: None,
            removed: vec![],
            applied: None,
            moved: vec![],
            notices: vec![],
            recalc: None,
        }
    }

    /// Rule 7: `base` (revision `rev`) → `ours` merged into the head
    /// `theirs`, which re-imports made from `base`. Our changes land as
    /// exact spans in the head, or the edit is refused where both changed.
    #[allow(clippy::too_many_arguments)]
    fn rebase(
        &self,
        doc: &DocRecord,
        rev: u32,
        over: &[u32],
        base: &str,
        ours: &str,
        theirs: &str,
        rem: &Remainder,
    ) -> Result<Next> {
        let hunks: Vec<Hunk> = match merge::merge(base, ours, theirs) {
            Merged::Clean(h) => h,
            Merged::Conflict(c) => {
                let shown: Vec<String> =
                    c.iter().map(|(a, b)| if a == b { format!("{a}") } else { format!("{a}–{b}") }).collect();
                let mut e = Error::new(
                    Code::MergeConflict,
                    format!(
                        "a person's edits (re-imported as revision {}) change the same lines of revision {rev} as this edit: lines {}. Read revision {} and edit again.",
                        over.last().unwrap(),
                        shown.join(", "),
                        doc.head
                    ),
                );
                e.detail.conflicts = c.iter().map(|&(a, b)| Conflict { first_line: a, last_line: b }).collect();
                e.detail.head = Some(doc.head);
                return Err(e);
            }
        };
        let mut next = match doc.format.text_model() {
            Some(m) => {
                // Last first, so the earlier spans stay where they are.
                let last_first: Vec<&Hunk> = hunks.iter().rev().collect();
                let (ra, removed) = chain(theirs, rem, last_first.len(), |k, t, r| {
                    let h = last_first[k];
                    let ra = reanchor_span_in(m, r, t, h.start, h.end, &h.text, doc.format.caps())?;
                    if ra.report.refused.is_empty() {
                        Ok(ra)
                    } else {
                        Err(hanji_core::Refusal::Unplaceable(ra.report).into())
                    }
                })?;
                match ra {
                    Some(ra) => self.finish(doc, theirs, ra, removed)?,
                    None => Next { text: theirs.to_string(), rem: rem.clone(), changed: self.changed(doc, true) },
                }
            }
            None => self.structure(doc, theirs, &merge::apply(theirs, &hunks), rem)?,
        };
        next.changed.rebased_over = over.to_vec();
        Ok(next)
    }

    fn store(&mut self, doc: &mut DocRecord, op: RevOp, summary: String, next: Next) -> Result<Changed> {
        let mut c = next.changed;
        if c.unchanged {
            return Ok(c);
        }
        c.parent = doc.head;
        c.revision = self.commit(doc, op, summary, &next.text, &next.rem)?;
        Ok(c)
    }
}

fn edit_summary(edits: &[TextEdit]) -> String {
    let clip = |s: &str| hanji_package::clip(s, 30);
    match edits {
        [e] => format!("edit {:?} → {:?}", clip(&e.old), clip(&e.new)),
        _ => format!("{} edits", edits.len()),
    }
}

/// Re-anchoring steps applied in turn, each to the text and remainder the
/// one before made: the last result (`None` for no steps) and every entry
/// removed on the way.
fn chain(
    text: &str,
    rem: &Remainder,
    steps: usize,
    mut step: impl FnMut(usize, &str, &Remainder) -> Result<Reanchored>,
) -> Result<(Option<Reanchored>, Vec<Loss>)> {
    let (mut cur, mut removed): (Option<Reanchored>, Vec<Loss>) = (None, vec![]);
    for k in 0..steps {
        let (t, r) = cur.as_ref().map_or((text, rem), |c| (c.text.as_str(), &c.remainder));
        let ra = step(k, t, r)?;
        removed.extend(losses(&ra.report));
        cur = Some(ra);
    }
    Ok((cur, removed))
}

#[cfg(not(target_family = "wasm"))]
mod native {
    use std::path::Path;

    use super::*;

    fn read_file(path: &Path) -> Result<Vec<u8>> {
        std::fs::read(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Error::not_found(format!("there is no file {}.", path.display())),
            _ => Error::io(format!("cannot read {}: {e}", path.display())),
        })
    }

    fn name_of(path: &Path) -> String {
        path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned())
    }

    impl<S: Storage> Workspace<S> {
        /// Open a file: import it (§8 neutralisation runs) as revision 1 of a new document.
        pub fn open(&mut self, path: &Path) -> Result<Opened> {
            let bytes = read_file(path)?;
            let source = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
            self.open_bytes(&name_of(path), &bytes, Some(source.display().to_string()))
        }

        /// A new file of `ty`, from a blank package or the template at `template`.
        pub fn create_from(&mut self, ty: DocType, format: Option<Format>, template: Option<&Path>) -> Result<Opened> {
            match template {
                None => self.create(ty, format, None),
                Some(p) => {
                    let bytes = read_file(p)?;
                    let format = format.or_else(|| Format::of_name(&name_of(p)));
                    self.create(ty, format, Some((&p.display().to_string(), &bytes)))
                }
            }
        }

        /// Export a revision to `path` (see [`Workspace::export_bytes`]).
        pub fn export(&self, id: &str, revision: Option<u32>, path: &Path, opts: &ExportOptions) -> Result<Exported> {
            let (mut out, bytes) = self.export_bytes(id, revision, opts)?;
            // The extension must be the format's own: an export carries no macros.
            let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());
            if ext.as_deref().is_some_and(|e| e != out.format.name() && Format::of_name(&format!("x.{e}")).is_some()) {
                return Err(Error::bad(format!(
                    "{id} exports as .{}; {} names another kind of file.",
                    out.format.name(),
                    path.display()
                )));
            }
            std::fs::write(path, &bytes).map_err(|e| Error::io(format!("cannot write {}: {e}", path.display())))?;
            out.path = Some(path.display().to_string());
            Ok(out)
        }

        /// Re-import the file at `path` (rule 7).
        pub fn reimport(&mut self, id: &str, path: &Path) -> Result<Reimported> {
            let bytes = read_file(path)?;
            self.reimport_bytes(id, &name_of(path), &bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert_eq!(slug("/tmp/3분기 보고서.docx"), "3분기-보고서");
        assert_eq!(slug("Q3 Report (final).DOCX"), "q3-report-final");
        assert_eq!(slug("...docx"), "doc");
        assert!(valid_id("3분기-보고서") && !valid_id("../x") && !valid_id("a/b") && !valid_id(""));
    }

    #[test]
    fn locating_old() {
        assert_eq!(locate("a b a", "b", 1).unwrap(), 2);
        let e = locate("x\nab\nab\n", "ab", 1).unwrap_err();
        assert_eq!((e.code, e.detail.matches, e.detail.lines.clone()), (Code::AmbiguousMatch, Some(2), vec![2, 3]));
        assert!(e.message.contains("occurs 2 times"), "{}", e.message);
        let e = locate("| a | b |\n", "|  a |  b |", 1).unwrap_err();
        assert_eq!(e.code, Code::NoMatch);
        assert!(e.message.contains("spaces and line breaks"), "{}", e.message);
        assert_eq!(locate("aaa", "aa", 1).unwrap_err().detail.matches, Some(2));
    }

    #[test]
    fn a_missing_old_names_the_look_alike_characters_of_the_revision() {
        let e = locate("# 계획\n\n□\u{2007}추진 배경\n", "□ 추진 배경", 1).unwrap_err();
        assert_eq!(e.code, Code::NoMatch);
        assert!(e.message.contains("matches line 3"), "{}", e.message);
        assert!(e.message.contains("\"□⟨U+2007 FIGURE SPACE⟩추진 배경\""), "{}", e.message);
        assert!(e.message.contains("\\u2007 in a JSON string"), "{}", e.message);
        let e = locate("\u{f076}2025년 계획\n다음 줄\n", " 2025년 계획\n다음", 1).unwrap_err();
        assert!(e.message.contains("⟨U+F076 private use⟩"), "{}", e.message);
        // A first line quoted from the revision is named too.
        let e = locate("□\u{2007}추진\n가\n", "□\u{2007}추진\n나", 1).unwrap_err();
        assert!(e.message.contains("Its first line \"□⟨U+2007 FIGURE SPACE⟩추진\" is at line 1"), "{}", e.message);
        assert!(!e.message.contains('\u{2007}') && !e.message.contains("\\u{2007}"), "{}", e.message);
    }
}
