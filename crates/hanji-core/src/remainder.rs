//! The remainder store (DESIGN.md §10.3): a flat list of typed entries per
//! revision, the namespace map once per document, and the package parts the
//! engine copies through.

use crate::model::{Path, StyleSet};
use hanji_format::{Keep, Marks};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Inline object shown as `<keep/>`. Anchor: its placeholder.
    Keep,
    /// Block shown as `<keep/>`. Anchor: its placeholder.
    Bkeep,
    /// Paragraph properties. Anchor: the paragraph.
    Ppr,
    /// Direct formatting of one run. Anchor: `[start, end)`.
    Run,
    /// Zero-width inline element (bookmark, comment range, …). Anchor: a position.
    Marker,
    /// Zero-width element inside a run. Anchor: a position (+ its run).
    Rmarker,
    /// Inline wrapper whose text the model edits (hyperlink, smartTag, …). Anchor: `[start, end)`.
    Wrap,
    /// Zero-width element between blocks, table rows, cells or cell
    /// paragraphs. Anchor: before the block, row, cell or paragraph at its
    /// path (a path one past the last means after it).
    Bmarker,
    /// Table, row and cell properties. Anchor: the table / row / cell.
    Tbl,
    Tr,
    Tc,
    /// After the last block (the body's final section properties). Anchor: the document.
    Tail,
}

impl Kind {
    /// Kinds anchored by a character offset or range.
    pub fn is_offset(self) -> bool {
        matches!(self, Kind::Run | Kind::Marker | Kind::Rmarker | Kind::Wrap)
    }
}

/// What the placer needs to know about an entry beyond its anchor.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    /// Placeholder shown for this entry (`Keep` / `Bkeep`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep: Option<Keep>,
    /// The run entry an in-run keep or `Rmarker` lives in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<u64>,
    /// Engine element name (a marker's tag).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub tag: String,
    /// A marker that means something to a person (bookmark, comment
    /// range): relocated, never dropped, when its paragraph goes.
    pub durable: bool,
    /// A range start: keeps right affinity (does not grow at its edge).
    pub opens: bool,
    /// Sequence number of a wrapper's end.
    pub seq_close: u64,
    /// Style id at import (`Ppr`, `Tbl`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Marks the run had at import (`Run`).
    pub marks: Marks,
    /// Engine data (docx: the run's original `w:t` elements).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aux: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: u64,
    pub kind: Kind,
    /// XML fragments, prefixes resolved by the document's namespace map.
    pub xml: Vec<String>,
    /// Canonical fingerprint of what the model does not represent; empty
    /// when the entry holds nothing unmodelled.
    pub fp: String,
    pub path: Path,
    pub start: Option<usize>,
    pub end: Option<usize>,
    /// Document order at import; orders zero-width things at one position.
    pub seq: u64,
    pub meta: Meta,
}

impl Entry {
    pub fn is_trivial(&self) -> bool {
        self.fp.is_empty()
    }
}

/// A package part copied through unchanged.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    #[serde(skip)]
    pub data: Vec<u8>,
    /// Zip metadata to reproduce the entry: DOS date/time, external attributes, stored or deflated.
    pub dos_time: u32,
    pub external_attr: u32,
    pub deflate: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remainder {
    /// Home format (`docx`).
    pub format: String,
    /// The document's namespace declarations, stored once: `(prefix, uri)`.
    pub namespaces: Vec<(String, String)>,
    /// Engine-defined document shell (docx: XML declaration, root and body
    /// start tags, root children around the body).
    pub shell: Vec<String>,
    pub styles: StyleSet,
    pub entries: Vec<Entry>,
    /// Every package part, in package order; the split part is a placeholder.
    pub parts: Vec<Part>,
    pub next_id: u64,
}

impl Remainder {
    pub fn keeps(&self) -> impl Iterator<Item = (&Keep, bool)> {
        self.entries.iter().filter_map(|e| e.meta.keep.as_ref().map(|k| (k, e.kind == Kind::Bkeep)))
    }

    pub fn keep_list(&self) -> Vec<Keep> {
        self.keeps().map(|(k, _)| k.clone()).collect()
    }

    pub fn is_block_keep(&self, id: &str) -> Option<bool> {
        self.keeps().find(|(k, _)| k.id == id).map(|(_, b)| b)
    }

    pub fn keep(&self, id: &str) -> Option<&Keep> {
        self.keeps().find(|(k, _)| k.id == id).map(|(k, _)| k)
    }

    /// The entry list as JSON lines (one entry per line, for inspection).
    pub fn entries_jsonl(&self) -> String {
        self.entries.iter().map(|e| serde_json::to_string(e).unwrap() + "\n").collect()
    }
}
