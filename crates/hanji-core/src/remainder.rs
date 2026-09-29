//! The remainder store (DESIGN.md §10.3): a flat list of typed entries per
//! revision, the namespace map once per document, and the package parts the
//! engine copies through.

use crate::model::{ListItem, Path, StyleSet};
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
    /// Everything of a slide but its slots' and shapes' text: its part, the
    /// objects on it the text does not show, its notes page. Anchor: the
    /// slide's head.
    Slide,
    /// A slot's or shape's own element, without its paragraphs. Anchor: its head.
    Shape,
}

impl Kind {
    /// Kinds anchored by a character offset or range.
    pub fn is_offset(self) -> bool {
        matches!(self, Kind::Run | Kind::Marker | Kind::Rmarker | Kind::Wrap)
    }
}

/// What the placer needs to know about an entry beyond its anchor.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
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
    /// The list item the paragraph was at import (`Ppr`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<ListItem>,
    /// Engine data (docx: the run's original `w:t` elements).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aux: Vec<String>,
    /// The package part whose relationships the entry's XML refers to
    /// (`r:` attributes): the entry cannot be placed in another part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
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

/// 64-bit FNV-1a: a stable hash (the same in every build and on wasm).
pub fn fnv1a(bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes.into_iter().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

/// Placeholder ids, stable across imports of the same file: `k` and four
/// base-32 digits of a hash of the object's content; the same object again
/// gets `-2`, `-3`, … in document order.
#[derive(Clone, Debug, Default)]
pub struct KeepIds(std::collections::HashSet<String>);

impl KeepIds {
    /// A fresh id for the object with fingerprint `fp`.
    pub fn next(&mut self, fp: &str) -> String {
        let h = fnv1a(fp.bytes());
        let base: String = std::iter::once('k')
            .chain((0..4).map(|k| char::from_digit(((h >> (k * 5)) & 31) as u32, 32).unwrap()))
            .collect();
        if self.0.insert(base.clone()) {
            return base;
        }
        (2..).map(|k| format!("{base}-{k}")).find(|id| self.0.insert(id.clone())).unwrap()
    }

    /// Give back an id (an import step that was rolled back).
    pub fn release(&mut self, id: &str) {
        self.0.remove(id);
    }
}

/// A package part copied through unchanged.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    /// The part's bytes (base64 in JSON).
    #[serde(with = "base64")]
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
    /// The whole remainder as JSON: everything export needs, package parts included.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a remainder serializes")
    }

    pub fn from_json(s: &str) -> Result<Remainder, String> {
        serde_json::from_str(s).map_err(|e| format!("not a remainder: {e}"))
    }

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

/// Standard base64 (RFC 4648, padded) for part bytes.
mod base64 {
    use serde::{de::Error, Deserialize, Deserializer, Serializer};

    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(data: &[u8]) -> String {
        let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
        for c in data.chunks(3) {
            let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
            for k in 0..4 {
                out.push(if k <= c.len() { ABC[(n >> (18 - 6 * k) & 63) as usize] as char } else { '=' });
            }
        }
        out
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        let s = s.as_bytes();
        if s.len() % 4 != 0 {
            return Err("base64 length is not a multiple of 4".into());
        }
        let val = |b: u8| ABC.iter().position(|&x| x == b).map(|v| v as u32);
        let mut out = Vec::with_capacity(s.len() / 4 * 3);
        for (q, c) in s.chunks(4).enumerate() {
            let pad = c.iter().rev().take_while(|&&b| b == b'=').count();
            if pad > 2 || (pad > 0 && q + 1 != s.len() / 4) {
                return Err("misplaced base64 padding".into());
            }
            let mut n = 0;
            for &b in &c[..4 - pad] {
                n = n << 6 | val(b).ok_or("not a base64 character")?;
            }
            n <<= 6 * pad as u32;
            out.extend_from_slice(&[(n >> 16) as u8, (n >> 8) as u8, n as u8][..3 - pad]);
        }
        Ok(out)
    }

    pub fn serialize<S: Serializer>(data: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&encode(data))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        decode(&String::deserialize(d)?).map_err(D::Error::custom)
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn roundtrip() {
            for (raw, enc) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foobar", "Zm9vYmFy")] {
                assert_eq!(super::encode(raw.as_bytes()), enc);
                assert_eq!(super::decode(enc).unwrap(), raw.as_bytes());
            }
            let all: Vec<u8> = (0..=255).collect();
            assert_eq!(super::decode(&super::encode(&all)).unwrap(), all);
            assert!(super::decode("Zg=a").is_err() && super::decode("Zm9").is_err());
        }
    }
}
