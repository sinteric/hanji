//! Cell content changes between revisions. Structure has its own text diff;
//! values, formulas and formula caches live in the remainder instead.
//!
//! Compare populated rows one pair at a time, never a rectangle of the used
//! range or a text rendering of the entire workbook. Only shared-formula
//! masters are indexed. The report is bounded even when every cell changes.

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::iter::Peekable;

use hanji_core::cells::col_letters;
use hanji_core::{EngineError, Remainder};
use hanji_format::formula;
use hanji_package::{package, xml};
use serde::Serialize;

use crate::book::{Book, SheetKind};
use crate::store::{Cell, Store};
use crate::{book_of, pkg, shift, sst};

/// Most changed cells included in a report; all changes are still counted.
pub const MAX_CHANGES: usize = 100;
/// Most bytes of each old/new cell state included before a truncation marker.
pub const MAX_STATE_BYTES: usize = 256;

#[derive(Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Value {
    #[default]
    Empty,
    // Keep the source number, rather than rounding it through f64.
    Number(String),
    Text(String),
    Bool(bool),
    Error(String),
    Date(String),
    Raw {
        kind: String,
        value: Option<String>,
    },
}

#[derive(PartialEq, Eq, Serialize)]
struct Formula {
    text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    attributes: Vec<(String, String)>,
}

#[derive(Default, PartialEq, Eq, Serialize)]
struct Content {
    /// For a formula this is its stored cache, including a missing cache.
    value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    formula: Option<Formula>,
}

type Masters = BTreeMap<String, (u32, u32, String)>;

fn shared_text(text: &str, dr: i64, dc: i64) -> String {
    if dr == 0 && dc == 0 {
        return text.into();
    }
    formula::rewrite_refs(text, &mut |token, _| {
        // Copying a shared formula adjusts external relative references too.
        // The mutation helper intentionally skips external prefixes, so pass
        // only the address body to it and retain the original prefix here.
        let prefix = formula::prefix_text(text, token);
        let address = &text[token.start + prefix.len()..token.end];
        Some(format!("{prefix}{}", shift::translate(address, dr, dc)))
    })
}

fn masters(store: &Store) -> Result<Masters, String> {
    let mut out = BTreeMap::new();
    for r in store.row_numbers() {
        let row = store.try_row(r)?.expect("an indexed row exists");
        for c in &row.cells {
            let Some(f) = &c.f else { continue };
            if f.get("t").as_deref() == Some("shared") && f.get("ref").is_some() {
                let si = f.get("si").ok_or("a shared formula has no si")?;
                if out.insert(si.clone(), (r, c.col, c.formula().unwrap_or_default())).is_some() {
                    return Err(format!("shared formula {si:?} has more than one master"));
                }
            }
        }
    }
    Ok(out)
}

fn content(book: &Book, shared: &Masters, row: u32, cell: Option<&Cell>) -> Result<Content, String> {
    let Some(c) = cell else { return Ok(Content::default()) };
    let v = c.v.as_deref().map(xml::unescape);
    let raw = || Value::Raw { kind: c.ty().into(), value: v.clone() };
    let value = match c.ty() {
        "s" => match v.as_deref().and_then(|v| v.trim().parse::<usize>().ok()) {
            Some(i) => book.sst.as_ref().and_then(|s| s.get(i)).map(|v| Value::Text(v.into())).unwrap_or_else(raw),
            None if v.is_none() => Value::Empty,
            None => raw(),
        },
        "str" => v.as_deref().map(|v| Value::Text(sst::decode_xstring(v))).unwrap_or_default(),
        "inlineStr" => c.is.as_ref().map(|v| Value::Text(sst::rich_text(v))).unwrap_or_default(),
        "b" => match v.as_deref().map(str::trim) {
            None => Value::Empty,
            Some(v) if v == "1" || v.eq_ignore_ascii_case("true") => Value::Bool(true),
            Some(v) if v == "0" || v.eq_ignore_ascii_case("false") => Value::Bool(false),
            _ => raw(),
        },
        "e" => v.map(Value::Error).unwrap_or_default(),
        "d" => v.map(Value::Date).unwrap_or_default(),
        "n" => match v.as_deref().map(str::trim) {
            None | Some("") => Value::Empty,
            Some(v) => Value::Number(v.into()),
        },
        _ => raw(),
    };
    let formula = if let Some(f) = &c.f {
        let shared_formula = f.get("t").as_deref() == Some("shared");
        let text = if shared_formula {
            let si = f.get("si").ok_or("a shared formula has no si")?;
            let (r, col, text) = shared
                .get(&si)
                .ok_or_else(|| format!("shared formula {si:?} at {}{row} has no master", col_letters(c.col)))?;
            shared_text(text, i64::from(row) - i64::from(*r), i64::from(c.col) - i64::from(*col))
        } else {
            c.formula().unwrap_or_default()
        };
        // Shared and expanded formulas have the same meaning. Keep other
        // attributes (in particular an array/data-table formula's range).
        let mut attributes: Vec<_> = f
            .attrs
            .iter()
            .filter(|(k, v)| !(shared_formula && matches!(k.as_str(), "t" | "si" | "ref") || k == "t" && v == "normal"))
            .map(|(k, v)| (k.clone(), xml::unescape(v)))
            .collect();
        attributes.sort();
        Some(Formula { text: format!("={text}"), attributes })
    } else {
        None
    };
    Ok(Content { value, formula })
}

/// A serializer sink that retains only a prefix, while hashing every byte.
/// Even one huge cell cannot cause an equally huge report allocation.
struct Prefix {
    bytes: Vec<u8>,
    total: usize,
    hash: u64,
}

impl Write for Prefix {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let keep = buf.len().min(MAX_STATE_BYTES - self.bytes.len());
        self.bytes.extend_from_slice(&buf[..keep]);
        self.total += buf.len();
        for &b in buf {
            self.hash = (self.hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn shown(value: &(impl Serialize + ?Sized)) -> String {
    let mut out = Prefix { bytes: vec![], total: 0, hash: 0xcbf2_9ce4_8422_2325 };
    serde_json::to_writer(&mut out, value).expect("cell content serializes to an infallible sink");
    // The prefix may end in the middle of a UTF-8 character.
    while std::str::from_utf8(&out.bytes).is_err() {
        out.bytes.pop();
    }
    let mut text = String::from_utf8(out.bytes).expect("the prefix is UTF-8");
    if out.total > MAX_STATE_BYTES {
        text.push_str(&format!("… [truncated; {} bytes; fnv1a64:{:016x}]", out.total, out.hash));
    }
    text
}

fn next_number<A: Iterator<Item = u32>, B: Iterator<Item = u32>>(
    a: &mut Peekable<A>,
    b: &mut Peekable<B>,
) -> Option<u32> {
    let next = match (a.peek(), b.peek()) {
        (Some(a), Some(b)) => *a.min(b),
        (Some(n), None) | (None, Some(n)) => *n,
        (None, None) => return None,
    };
    a.next_if_eq(&next);
    b.next_if_eq(&next);
    Some(next)
}

fn sheet(book: &Book, index: Option<usize>) -> Result<Option<Store>, String> {
    index
        .map(|i| {
            let part = &book.sheets[i].part;
            let data = package::get(&book.parts, part).ok_or_else(|| format!("no {part}"))?;
            Store::load(data.to_vec()).map_err(|e| format!("{part}: {e}"))
        })
        .transpose()
}

pub(crate) fn cells(before: &Remainder, after: &Remainder) -> Result<String, EngineError> {
    let (a, b) = (book_of(before)?, book_of(after)?);
    // Names identify sheets, independently of their order and part names.
    // A renamed sheet appears as removed/added; its rename is in the text diff.
    let mut sheets: BTreeMap<&str, (Option<usize>, Option<usize>)> = BTreeMap::new();
    for (i, s) in a.sheets.iter().enumerate().filter(|(_, s)| s.kind == SheetKind::Work) {
        if sheets.entry(&s.name).or_default().0.replace(i).is_some() {
            return Err(pkg(format!("more than one worksheet named {:?}", s.name)));
        }
    }
    for (i, s) in b.sheets.iter().enumerate().filter(|(_, s)| s.kind == SheetKind::Work) {
        if sheets.entry(&s.name).or_default().1.replace(i).is_some() {
            return Err(pkg(format!("more than one worksheet named {:?}", s.name)));
        }
    }
    let mut count = 0;
    let mut changes = String::new();
    for (name, (ai, bi)) in sheets {
        // Drop each pair after comparison instead of keeping all sheets parsed.
        let (ast, bst) = (sheet(&a, ai).map_err(pkg)?, sheet(&b, bi).map_err(pkg)?);
        let am = ast.as_ref().map(masters).transpose().map_err(pkg)?.unwrap_or_default();
        let bm = bst.as_ref().map(masters).transpose().map_err(pkg)?.unwrap_or_default();
        let mut ar = ast.iter().flat_map(Store::row_numbers).peekable();
        let mut br = bst.iter().flat_map(Store::row_numbers).peekable();
        while let Some(row) = next_number(&mut ar, &mut br) {
            let old = ast.as_ref().map(|s| s.try_row(row)).transpose().map_err(pkg)?.flatten();
            let new = bst.as_ref().map(|s| s.try_row(row)).transpose().map_err(pkg)?.flatten();
            let mut ac = old.iter().flat_map(|r| r.cells.iter().map(|c| c.col)).peekable();
            let mut bc = new.iter().flat_map(|r| r.cells.iter().map(|c| c.col)).peekable();
            while let Some(col) = next_number(&mut ac, &mut bc) {
                let old = content(&a, &am, row, old.as_ref().and_then(|r| r.cell(col))).map_err(pkg)?;
                let new = content(&b, &bm, row, new.as_ref().and_then(|r| r.cell(col))).map_err(pkg)?;
                if old == new {
                    continue;
                }
                count += 1;
                if count <= MAX_CHANGES {
                    changes.push_str(&format!(
                        "@@ {}!{}{row} @@\n- {}\n+ {}\n",
                        shown(name),
                        col_letters(col),
                        shown(&old),
                        shown(&new)
                    ));
                }
            }
        }
    }
    if count == 0 {
        return Ok(String::new());
    }
    let mut out = format!("Cell changes: {count} (value is the stored cache when formula is present)\n");
    out.push_str(&changes);
    if count > MAX_CHANGES {
        out.push_str(&format!(
            "{} more changed cells omitted (first {MAX_CHANGES} shown in sheet/row/column order).\n",
            count - MAX_CHANGES
        ));
    }
    Ok(out)
}
