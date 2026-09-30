//! The workbook's structure as text (§5.4) derived from a [`Book`], and the
//! remainder's range entries: the worksheet elements anchored to cells
//! (merged cells, conditional formats, data validations, hyperlinks) taken
//! out of each worksheet at import and put back at export, at their anchors.

use std::collections::{BTreeMap, HashMap};

use hanji_core::cells::{parse_sqref, CellRange, CellRef};
use hanji_core::{Entry, KeepIds, Kind, Meta};
use hanji_format::cellfmt::{self, CellFormat, FormatLine};
use hanji_format::formula;
use hanji_format::sheet::{format_kind, ColumnDecl, FormatKind, SheetDecl, SheetItem, Spreadsheet, TableDecl};
use hanji_format::{FrontMatter, Keep};
use hanji_package::clip;
use hanji_package::xml::{self, Element, Node};

use crate::book::{Book, SheetKind, TableInfo, ValueKind};
use crate::drawing;
use crate::store::SHEET_ORDER;

/// The worksheet children that become range entries: (container, child, anchor attribute).
pub const RANGE_ELEMENTS: &[(Option<&str>, &str, &str)] = &[
    (Some("mergeCells"), "mergeCell", "ref"),
    (None, "conditionalFormatting", "sqref"),
    (Some("dataValidations"), "dataValidation", "sqref"),
    (Some("hyperlinks"), "hyperlink", "ref"),
];

/// The placeholder kind of a range entry's element.
pub fn keep_kind(tag: &str) -> &'static str {
    match tag {
        "mergeCell" => "merged-cells",
        "conditionalFormatting" => "conditional-format",
        "dataValidation" => "data-validation",
        _ => "hyperlinks",
    }
}

/// Take the range elements out of worksheet `i`'s skeleton into entries.
pub fn extract_entries(book: &mut Book, i: usize, next_id: &mut u64) -> Result<(), String> {
    let part = book.sheets[i].part.clone();
    let st = book.store_mut(i)?;
    let scope = xml::scope_of(&st.skel.root);
    let mut found: Vec<Entry> = vec![];
    let mut containers: BTreeMap<String, String> = BTreeMap::new();
    let mut seq = 0;
    let root = &mut st.skel.root;
    let mut kept = vec![];
    for n in std::mem::take(&mut root.children) {
        let Node::El(e) = &n else {
            kept.push(n);
            continue;
        };
        let l = e.local();
        let spec = RANGE_ELEMENTS.iter().find(|(c, child, _)| c.map_or(*child == l, |c| c == l));
        let Some(&(container, child, anchor)) = spec else {
            kept.push(n);
            continue;
        };
        let items: Vec<Element> = if container.is_some() {
            containers.insert(l.to_string(), e.shell().to_xml());
            e.elements().filter(|x| x.local() == child).cloned().collect()
        } else {
            vec![e.clone()]
        };
        if container.is_some() && e.elements().any(|x| x.local() != child) {
            return Err(format!("{part}: <{l}> holds elements other than <{child}>"));
        }
        for it in items {
            let range = it.get(anchor).unwrap_or_default();
            if parse_sqref(&range).is_none() {
                return Err(format!("{part}: <{child}> has no cell range ({anchor}=\"{range}\")"));
            }
            found.push(Entry {
                id: *next_id,
                kind: Kind::Range,
                xml: vec![it.to_xml()],
                fp: xml::canon(&it, &scope),
                path: vec![i],
                start: None,
                end: None,
                seq,
                meta: Meta { tag: child.to_string(), range: Some(range), ..Default::default() },
            });
            *next_id += 1;
            seq += 1;
        }
    }
    root.children = kept;
    if !containers.is_empty() {
        book.shell.containers.insert(part, containers);
    }
    book.entries.extend(found);
    Ok(())
}

/// Put worksheet `i`'s range entries back into its skeleton, at their current ranges.
pub fn restore_entries(book: &mut Book, i: usize) -> Result<(), String> {
    let part = book.sheets[i].part.clone();
    let mine: Vec<Entry> =
        book.entries.iter().filter(|e| e.kind == Kind::Range && e.path.first() == Some(&i)).cloned().collect();
    if mine.is_empty() {
        return Ok(());
    }
    let containers = book.shell.containers.get(&part).cloned().unwrap_or_default();
    let st = book.store_mut(i)?;
    let prefix = st.prefix.clone();
    for &(container, child, anchor) in RANGE_ELEMENTS {
        let els: Vec<Element> = mine
            .iter()
            .filter(|e| e.meta.tag == child)
            .map(|e| {
                let mut x = xml::fragment(&e.xml[0]);
                if let Some(r) = &e.meta.range {
                    if x.get(anchor).as_deref() != Some(r.as_str()) {
                        x.set(anchor, r);
                    }
                }
                x
            })
            .collect();
        if els.is_empty() {
            continue;
        }
        match container {
            Some(c) => {
                let mut ce = match containers.get(c) {
                    Some(tag) => xml::fragment(tag),
                    None => Element::new(&format!("{prefix}{c}")),
                };
                if ce.attr("count").is_some() || c == "mergeCells" {
                    ce.set("count", &els.len().to_string());
                }
                ce.children = els.into_iter().map(Node::El).collect();
                xml::insert_ordered(&mut st.skel.root, ce, SHEET_ORDER);
            }
            None => {
                // Several top-level elements: each after the last one placed.
                for x in els {
                    xml::insert_ordered(&mut st.skel.root, x, SHEET_ORDER);
                }
            }
        }
    }
    Ok(())
}

/// The placeholders of the book as it is now: one per kind of range entry
/// per sheet, and one per object the sheet shows. Ids are stable: a hash of
/// the sheet's part, the kind and its place.
pub fn keeps(book: &Book) -> Vec<(usize, Keep)> {
    let mut ids = KeepIds::default();
    let mut out = vec![];
    for (i, s) in book.sheets.iter().enumerate() {
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        let mut push = |kind: &str, summary: String, out: &mut Vec<(usize, Keep)>| {
            let n = seen.entry(kind.to_string()).or_insert(0);
            *n += 1;
            let id = ids.next(&format!("{}|{kind}|{n}", s.part));
            out.push((i, Keep { id, kind: kind.into(), summary: clip(&summary, 160) }));
        };
        for &(_, child, _) in RANGE_ELEMENTS {
            let ranges: Vec<&str> = book
                .entries
                .iter()
                .filter(|e| e.kind == Kind::Range && e.path.first() == Some(&i) && e.meta.tag == child)
                .filter_map(|e| e.meta.range.as_deref())
                .collect();
            if !ranges.is_empty() {
                push(keep_kind(child), list_summary(&ranges), &mut out);
            }
        }
        match s.kind {
            SheetKind::Work => {
                for o in drawing::sheet_objects(book, i) {
                    push(&o.kind, o.summary, &mut out);
                }
            }
            SheetKind::Chart => {
                let rels = book.rels_of(&s.part);
                for r in rels.iter().filter(|r| r.short_type() == "drawing") {
                    for o in drawing::drawing_objects(book, &hanji_package::opc::resolve_target(&s.part, &r.target)) {
                        push(&o.kind, o.summary, &mut out);
                    }
                }
            }
            SheetKind::Other(ref k) => push("sheet", format!("a {k} sheet"), &mut out),
        }
    }
    out
}

fn list_summary(ranges: &[&str]) -> String {
    let mut s = String::new();
    for (k, r) in ranges.iter().enumerate() {
        let next = if s.is_empty() { r.to_string() } else { format!("{s}, {r}") };
        if next.chars().count() > 120 {
            s.push_str(&format!(" (+{} more)", ranges.len() - k));
            return s;
        }
        s = next;
    }
    s
}

/// A table's columns: type and format from its cells (§8: typed by the
/// source), the formula from the table's calculated column.
pub fn table_decl(book: &Book, t: &TableInfo) -> TableDecl {
    let (first, last) = t.data_rows();
    let n = t.cols.len();
    let c0 = t.range.first.col;
    // Per column: texts, numbers, others; cells by style; the first cell's style.
    let mut kinds = vec![(0usize, 0usize, 0usize); n];
    let mut styles: Vec<HashMap<u32, usize>> = vec![HashMap::new(); n];
    let mut first_style: Vec<Option<u32>> = vec![None; n];
    if first <= last {
        book.store(t.sheet).for_each_row_in(first, last, &mut |r| {
            for cell in r.cells.iter().filter(|c| c.col >= c0 && ((c.col - c0) as usize) < n) {
                let k = (cell.col - c0) as usize;
                first_style[k].get_or_insert(cell.style());
                match book.kind(cell) {
                    ValueKind::Empty => continue,
                    ValueKind::Text => kinds[k].0 += 1,
                    ValueKind::Number => kinds[k].1 += 1,
                    ValueKind::Other => kinds[k].2 += 1,
                }
                *styles[k].entry(cell.style()).or_insert(0) += 1;
            }
        });
    }
    let mut columns = vec![];
    for (k, c) in t.cols.iter().enumerate() {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for (s, m) in &styles[k] {
            *counts.entry(book.styles.format_of(*s)).or_insert(0) += m;
        }
        let (texts, nums, others) = kinds[k];
        let format = counts
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(f, _)| f.clone())
            .or_else(|| c.dxf.and_then(|d| book.styles.dxf_fmt.get(&d).cloned()))
            .or_else(|| first_style[k].map(|s| book.styles.format_of(s)))
            .unwrap_or_else(|| "General".into());
        let kind = format_kind(&format);
        let ty = match (texts, nums, others) {
            (0, 0, 0) => match kind {
                FormatKind::Text => "text",
                FormatKind::Date => "date",
                FormatKind::Number => "number",
                FormatKind::General => "mixed",
            },
            (_, 0, 0) => "text",
            (0, _, 0) if kind == FormatKind::Date => "date",
            (0, _, 0) => "number",
            _ => "mixed",
        };
        let formula = c
            .calc
            .as_ref()
            .map(|f| formula::to_short_form(&format!("={}", f.trim_start_matches('=')), &t.name))
            .unwrap_or_default();
        columns.push(ColumnDecl { name: c.name.clone(), ty: ty.into(), format, formula });
    }
    TableDecl { name: t.name.clone(), range: t.range.to_string(), columns }
}

/// The structure text of the book (§5.4). Loads every worksheet.
pub fn structure(book: &mut Book, template: Option<&str>) -> Result<Spreadsheet, String> {
    for i in 0..book.sheets.len() {
        if book.sheets[i].kind == SheetKind::Work {
            book.load_store(i)?;
        }
    }
    let keeps = keeps(book);
    let normal = book.styles.normal_format();
    let shown = book.styles.part.is_some();
    let mut sheets = vec![];
    for (i, s) in book.sheets.iter().enumerate() {
        let mut items = vec![];
        let mut range = None;
        let mut formats = vec![];
        if s.kind == SheetKind::Work {
            if shown {
                formats = format_lines(book, i, &normal);
            }
            let st = book.store(i);
            range = st.used().map(|(c0, r0, c1, r1)| {
                CellRange::new(hanji_core::cells::CellRef::new(c0, r0), hanji_core::cells::CellRef::new(c1, r1))
                    .to_string()
            });
            for t in book.tables.iter().filter(|t| t.sheet == i) {
                items.push(SheetItem::Table(table_decl(book, t)));
            }
        }
        items.extend(keeps.iter().filter(|(k, _)| *k == i).map(|(_, k)| SheetItem::Keep(k.clone())));
        sheets.push(SheetDecl { name: s.name.clone(), range, formats, items });
    }
    let default_format = shown.then(|| normal.diff(&CellFormat::implicit()));
    Ok(Spreadsheet { front: FrontMatter::spreadsheet("xlsx", template), default_format, sheets })
}

/// Sheet `i`'s formatted cells (§5.4): one line per rectangle of cells with
/// the same formatting beyond Normal's and the same named style.
fn format_lines(book: &Book, i: usize, normal: &CellFormat) -> Vec<FormatLine> {
    let styles = &book.styles;
    let mut seen: HashMap<u32, Option<(Option<String>, CellFormat)>> = HashMap::new();
    let mut shown = |s: u32| -> Option<(Option<String>, CellFormat)> {
        seen.entry(s)
            .or_insert_with(|| {
                let f = styles.cell_format(s).diff(normal);
                let name = styles.style_name(s);
                (!f.is_empty() || name.is_some()).then_some((name, f))
            })
            .clone()
    };
    // Style 0 is usually Normal's: then only rows with a cell style are read.
    let all = shown(0).is_some();
    let mut cells: Vec<(u32, u32, (Option<String>, CellFormat))> = vec![];
    let pre = |row: &[u8]| all || crate::store::holds(row, b" s=\"");
    book.store(i).for_each_cell_if(&pre, &mut |r, c| {
        if let Some(v) = shown(c.style()) {
            cells.push((r, c.col, v));
        }
    });
    cellfmt::rectangles(&cells)
        .into_iter()
        .map(|(r1, c1, r2, c2, (style, format))| FormatLine {
            range: CellRange::new(CellRef::new(c1, r1), CellRef::new(c2, r2)).to_string(),
            style,
            format,
        })
        .collect()
}

/// Placeholder entries for `rem.entries` (so the text's keeps can be checked).
pub fn keep_entries(book: &Book, next_id: &mut u64) -> Vec<Entry> {
    keeps(book)
        .into_iter()
        .map(|(i, k)| {
            let e = Entry {
                id: *next_id,
                kind: Kind::Bkeep,
                xml: vec![],
                fp: String::new(),
                path: vec![i],
                start: None,
                end: None,
                seq: 0,
                meta: Meta { tag: k.kind.clone(), keep: Some(k), ..Default::default() },
            };
            *next_id += 1;
            e
        })
        .collect()
}
