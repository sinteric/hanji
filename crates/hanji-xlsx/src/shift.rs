//! Rows moving in a span of columns (inserted, deleted, sorted) and
//! everything anchored to them: the cells, every formula that refers to them
//! (on any sheet, in defined names, in conditional formats and validations,
//! in charts), the range entries, the tables, notes, drawings, pivot tables
//! and filters. [`check`] refuses, with the reason, a move that would tear
//! something the engine cannot re-anchor; [`apply`] moves everything and
//! reports what went with deleted rows.

use std::collections::{BTreeMap, BTreeSet};

use hanji_core::cells::{parse_sqref, write_sqref, CellRange, CellRef, Moved, RowShift, MAX_ROW};
use hanji_core::{notice, Kind, Notice};
use hanji_format::formula::{self, Corner, Reference, Tok};
use hanji_package::opc;
use hanji_package::package;
use hanji_package::xml::{self, Element, Node};

use crate::book::{Book, SheetKind};
use crate::drawing;
use crate::store::{has_formula, holds, Cell, Row};

/// What a formula reference points at: its sheet and area (whole rows or
/// columns are not areas here).
fn ref_area(r: &Reference) -> Option<CellRange> {
    let a = r.first?;
    let b = r.last.unwrap_or(a);
    Some(CellRange::new(CellRef::new(a.col?, a.row?), CellRef::new(b.col?, b.row?)))
}

fn with_rows(c: Corner, row: u32) -> Corner {
    Corner { row: Some(row), ..c }
}

fn write_ref(prefix: &str, r: &Reference, to: CellRange) -> String {
    let (a, b) = (r.first.unwrap(), r.last);
    // Corners keep their own `$` flags; which corner is which follows the written order.
    let (top_first, left_first) = match b {
        Some(b) => (a.row <= b.row, a.col <= b.col),
        None => (true, true),
    };
    let _ = left_first;
    let first = with_rows(a, if top_first { to.first.row } else { to.last.row });
    match b {
        None => format!("{prefix}{first}"),
        Some(b) => format!("{prefix}{first}:{}", with_rows(b, if top_first { to.last.row } else { to.first.row })),
    }
}

/// `f` (a formula on sheet `own`) with its references into sheet `target`
/// moved by `sh`; `None` when nothing changes.
pub fn shift_formula(f: &str, own: &str, target: &str, sh: &RowShift) -> Option<String> {
    let mut changed = false;
    let out = formula::rewrite_refs(f, &mut |t, r| {
        let sheet = match &r.sheet {
            Some(p) if p.external || p.to.is_some() => return None,
            Some(p) => p.name.as_str(),
            None => own,
        };
        if !sheet.eq_ignore_ascii_case(target) {
            return None;
        }
        let area = ref_area(r)?;
        let prefix = formula::prefix_text(f, t);
        match sh.range(&area) {
            Moved::Same | Moved::Split => None,
            Moved::Gone => {
                changed = true;
                Some(format!("{prefix}#REF!"))
            }
            Moved::To(to) => {
                changed = true;
                Some(write_ref(prefix, r, to))
            }
        }
    });
    changed.then_some(out)
}

/// `f` as if copied `dr` rows and `dc` columns away: relative references
/// move, absolute ones stay; one pushed off the sheet becomes `#REF!`.
pub fn translate(f: &str, dr: i64, dc: i64) -> String {
    if dr == 0 && dc == 0 {
        return f.to_string();
    }
    formula::rewrite_refs(f, &mut |t, r| {
        if r.sheet.as_ref().is_some_and(|s| s.external) {
            return None;
        }
        let prefix = formula::prefix_text(f, t);
        let mv = |c: Corner| -> Option<Corner> {
            let col = match c.col {
                Some(x) if !c.abs_col => Some(u32::try_from(x as i64 + dc).ok().filter(|v| *v < 16_384)?),
                x => x,
            };
            let row = match c.row {
                Some(x) if !c.abs_row => Some(u32::try_from(x as i64 + dr).ok().filter(|v| *v >= 1 && *v <= MAX_ROW)?),
                x => x,
            };
            Some(Corner { col, row, ..c })
        };
        let first = r.first?;
        let a = mv(first);
        let b = r.last.map(mv);
        Some(match (a, b) {
            (Some(a), None) => format!("{prefix}{a}"),
            (Some(a), Some(Some(b))) => format!("{prefix}{a}:{b}"),
            _ => format!("{prefix}#REF!"),
        })
    })
}

fn shared_attr(f: &Element) -> Option<(String, Option<String>)> {
    (f.get("t").as_deref() == Some("shared")).then(|| (f.get("si").unwrap_or_default(), f.get("ref")))
}

/// A plain `<f>` with `text`.
pub fn f_element(prefix: &str, text: &str) -> Element {
    let mut e = Element::new(&format!("{prefix}f"));
    e.children.push(Node::Text(xml::escape_text(text)));
    e
}

/// Give every cell of sheet `i`'s shared formulas its own formula.
pub fn unshare(book: &mut Book, i: usize) -> Result<bool, String> {
    book.load_store(i)?;
    let mut masters: BTreeMap<String, (u32, u32, String)> = BTreeMap::new();
    let mut any = false;
    book.store(i).for_each_cell_if(&has_formula, &mut |r, c| {
        if let Some((si, rf)) = c.f.as_ref().and_then(shared_attr) {
            any = true;
            if rf.is_some() {
                masters.insert(si, (r, c.col, c.formula().unwrap_or_default()));
            }
        }
    });
    if !any {
        return Ok(false);
    }
    let rows: Vec<u32> = {
        let mut v = BTreeSet::new();
        book.store(i).for_each_cell_if(&has_formula, &mut |r, c| {
            if c.f.as_ref().and_then(shared_attr).is_some() {
                v.insert(r);
            }
        });
        v.into_iter().collect()
    };
    let st = book.store_mut(i)?;
    let prefix = st.prefix.clone();
    for r in rows {
        let row = st.row_mut(r);
        for c in &mut row.cells {
            let Some((si, _)) = c.f.as_ref().and_then(shared_attr) else { continue };
            let Some((mr, mc, text)) = masters.get(&si) else {
                return Err(format!("a shared formula at {}{r} has no master", hanji_core::cells::col_letters(c.col)));
            };
            let t = translate(text, r as i64 - *mr as i64, c.col as i64 - *mc as i64);
            let mut f = f_element(&prefix, &t);
            for (k, v) in &c.f.as_ref().unwrap().attrs {
                if !matches!(k.as_str(), "t" | "ref" | "si") {
                    f.attrs.push((k.clone(), v.clone()));
                }
            }
            c.f = Some(f);
        }
    }
    Ok(true)
}

/// The formula texts of the book that can refer to sheet `target`: `(sheet, row, col)`.
fn formula_cells_reading(book: &mut Book, target: &str) -> Result<Vec<(usize, u32, u32)>, String> {
    let mut out = vec![];
    for i in 0..book.sheets.len() {
        if book.sheets[i].kind != SheetKind::Work {
            continue;
        }
        book.load_store(i)?;
        let own = book.sheets[i].name.eq_ignore_ascii_case(target);
        let needle = target.as_bytes().to_vec();
        let pre = |b: &[u8]| has_formula(b) && (own || holds(b, &needle));
        book.store(i).for_each_cell_if(&pre, &mut |r, c| {
            if c.f.is_some() {
                out.push((i, r, c.col));
            }
        });
    }
    Ok(out)
}

/// Why `sh` on sheet `i` would tear something, if it would.
pub fn check(book: &mut Book, i: usize, sh: &RowShift, own_table: Option<usize>) -> Result<(), String> {
    let name = book.sheets[i].name.clone();
    let (c0, c1) = sh.cols();
    let span = format!("columns {}–{}", hanji_core::cells::col_letters(c0), hanji_core::cells::col_letters(c1));
    let why = |what: &str, r: &str| {
        format!("{what} {r} on sheet {name} reaches outside {span}, into rows that move; it would be torn in two")
    };
    for e in book.entries.iter().filter(|e| e.kind == Kind::Range && e.path.first() == Some(&i)) {
        for r in parse_sqref(e.meta.range.as_deref().unwrap_or("")).unwrap_or_default() {
            if sh.range(&r) == Moved::Split {
                return Err(why(&describe(&e.meta.tag), &r.to_string()));
            }
        }
    }
    for (k, t) in book.tables.iter().enumerate() {
        if t.sheet == i && Some(k) != own_table && sh.range(&t.range) == Moved::Split {
            return Err(why(&format!("table {}", t.name), &t.range.to_string()));
        }
    }
    book.load_store(i)?;
    let st = book.store(i);
    let mut err = None;
    st.for_each_cell_if(&has_formula, &mut |r, c| {
        let Some(f) = &c.f else { return };
        let at = format!("{}{r}", hanji_core::cells::col_letters(c.col));
        match f.get("t").as_deref() {
            Some("array") => {
                let rr =
                    f.get("ref").and_then(|x| CellRange::parse(&x)).unwrap_or(CellRange::cell(CellRef::new(c.col, r)));
                match sh.range(&rr) {
                    Moved::Same | Moved::Gone => {}
                    Moved::To(to) if to.rows() == rr.rows() => {}
                    _ => {
                        err = err.take().or(Some(format!(
                            "the array formula at {rr} on sheet {name} would be split by rows that move"
                        )))
                    }
                }
            }
            Some("dataTable") => {
                let rr =
                    f.get("ref").and_then(|x| CellRange::parse(&x)).unwrap_or(CellRange::cell(CellRef::new(c.col, r)));
                if sh.range(&rr) != Moved::Same {
                    err = err
                        .take()
                        .or(Some(format!("the data table at {rr} on sheet {name} is not modelled and would move")));
                }
            }
            _ => {
                let _ = at;
            }
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    if let Some(af) = st.child("autoFilter").and_then(|a| a.get("ref")).and_then(|r| CellRange::parse(&r)) {
        if sh.range(&af) == Moved::Split {
            return Err(why("the sheet's filter", &af.to_string()));
        }
    }
    // 2010 extensions (sparklines, x14 conditional formats and validations) carry ranges the engine does not model.
    let exts = drawing::extensions(&st.skel.root);
    if !exts.is_empty() {
        let mut hit = false;
        if let Some(ext) = st.child("extLst") {
            ext.walk(&mut |e| {
                if matches!(e.local(), "sqref" | "f") {
                    let t = e.text_of(&[e.name.as_str()]);
                    for part in t.split_whitespace() {
                        let part = part.rsplit('!').next().unwrap_or(part).replace('$', "");
                        if let Some(r) = CellRange::parse(&part) {
                            if sh.range(&r) != Moved::Same {
                                hit = true;
                            }
                        }
                    }
                }
            });
        }
        if hit {
            return Err(format!(
                "sheet {name} has {} that refer to rows that move; the engine does not model them",
                exts.join(", ")
            ));
        }
    }
    // Pivot tables on the sheet.
    for r in book.rels_of(&book.sheets[i].part).iter().filter(|r| r.short_type() == "pivotTable" && !r.external) {
        let part = opc::resolve_target(&book.sheets[i].part, &r.target);
        let Some(d) = package::get(&book.parts, &part).and_then(|d| xml::parse(d).ok()) else { continue };
        let loc = d
            .root
            .elements()
            .find(|e| e.local() == "location")
            .and_then(|l| l.get("ref"))
            .and_then(|x| CellRange::parse(&x));
        if let Some(loc) = loc {
            match sh.range(&loc) {
                Moved::Same => {}
                Moved::To(to) if to.rows() == loc.rows() => {}
                _ => return Err(format!("the pivot table at {loc} on sheet {name} would be torn by rows that move")),
            }
        }
    }
    Ok(())
}

pub fn describe(tag: &str) -> String {
    match tag {
        "mergeCell" => "the merged cells",
        "conditionalFormatting" => "the conditional format of",
        "dataValidation" => "the data validation of",
        "hyperlink" => "the hyperlink at",
        other => other,
    }
    .to_string()
}

/// Move rows of sheet `i` by `sh`, and everything anchored to them.
/// Returns what went with deleted rows.
pub fn apply(book: &mut Book, i: usize, sh: &RowShift, out: &mut Vec<Notice>) -> Result<(), String> {
    let name = book.sheets[i].name.clone();
    // Shared formulas that read the sheet become plain ones first.
    for k in 0..book.sheets.len() {
        if book.sheets[k].kind == SheetKind::Work {
            let reads = k == i || {
                book.load_store(k)?;
                let mut hit = false;
                let needle = name.as_bytes().to_vec();
                book.store(k).for_each_cell_if(&|b| has_formula(b) && holds(b, &needle), &mut |_, c| {
                    hit |= c.f.as_ref().and_then(shared_attr).is_some();
                });
                hit
            };
            if reads {
                unshare(book, k)?;
            }
        }
    }
    // Formulas anywhere that refer to the moving rows.
    for (k, r, col) in formula_cells_reading(book, &name)? {
        let own = book.sheets[k].name.clone();
        let st = book.store_mut(k)?;
        let row = st.row_mut(r);
        let Some(cell) = row.cells.iter_mut().find(|c| c.col == col) else { continue };
        if let Some(text) = cell.formula() {
            if let Some(new) = shift_formula(&text, &own, &name, sh) {
                let f = cell.f.as_mut().unwrap();
                f.children = vec![Node::Text(xml::escape_text(&new))];
            }
        }
        if k == i {
            if let Some(f) = cell.f.as_mut() {
                if let Some(rr) = f.get("ref").and_then(|x| CellRange::parse(&x)) {
                    if let Moved::To(to) = sh.range(&rr) {
                        f.set("ref", &to.to_string());
                    }
                }
            }
        }
    }
    move_cells(book, i, sh, out)?;
    // Range entries of the sheet, and the formulas of every range entry.
    let mut gone_ids = vec![];
    for e in book.entries.iter_mut().filter(|e| e.kind == Kind::Range) {
        let sheet_of = e.path.first().copied().unwrap_or(usize::MAX);
        if e.meta.tag != "hyperlink" {
            let own = book.sheets.get(sheet_of).map(|s| s.name.clone()).unwrap_or_default();
            let mut x = xml::fragment(&e.xml[0]);
            let mut changed = false;
            x.walk_mut(&mut |el| {
                if el.local() == "formula" || el.local() == "formula1" || el.local() == "formula2" {
                    let t = el.text_of(&[el.name.as_str()]);
                    if let Some(new) = shift_formula(&t, &own, &name, sh) {
                        el.children = vec![Node::Text(xml::escape_text(&new))];
                        changed = true;
                    }
                }
            });
            if changed {
                e.xml[0] = x.to_xml();
            }
        }
        if sheet_of != i {
            continue;
        }
        let ranges = parse_sqref(e.meta.range.as_deref().unwrap_or("")).unwrap_or_default();
        let mut kept = vec![];
        for r in &ranges {
            match sh.range(r) {
                Moved::Same | Moved::Split => kept.push(*r),
                Moved::To(to) => kept.push(to),
                Moved::Gone => {}
            }
        }
        if kept.is_empty() {
            notice(
                out,
                "removed",
                format!("{name}!{}", e.meta.range.clone().unwrap_or_default()),
                format!("{} went with its deleted rows", describe(&e.meta.tag)),
            );
            gone_ids.push(e.id);
        } else if kept != ranges {
            e.meta.range = Some(write_sqref(&kept));
        }
    }
    book.entries.retain(|e| !gone_ids.contains(&e.id));
    // Tables of the sheet.
    for t in book.tables.iter_mut().filter(|t| t.sheet == i) {
        if let Moved::To(to) = sh.range(&t.range) {
            t.range = to;
            t.changed = true;
        }
    }
    // Defined names.
    let mut wb_changed = false;
    if let Some(names) = book.wb.root.elements_mut().find(|e| e.local() == "definedNames") {
        for d in names.elements_mut() {
            let t = d.text_of(&[d.name.as_str()]);
            let own = String::new();
            if let Some(new) = shift_formula(&t, &own, &name, sh) {
                d.children = vec![Node::Text(xml::escape_text(&new))];
                wb_changed = true;
            }
        }
    }
    book.wb_changed |= wb_changed;
    // The sheet's own anchors: its filter, sort state, protected ranges and ignored errors.
    {
        let st = book.store_mut(i)?;
        for local in ["autoFilter", "sortState"] {
            if let Some(e) = st.child_mut(local) {
                if let Some(r) = e.get("ref").and_then(|x| CellRange::parse(&x)) {
                    if let Moved::To(to) = sh.range(&r) {
                        e.set("ref", &to.to_string());
                    }
                }
            }
        }
        for local in ["protectedRanges", "ignoredErrors"] {
            if let Some(e) = st.child_mut(local) {
                for c in e.elements_mut() {
                    if let Some(rs) = c.get("sqref").and_then(|x| parse_sqref(&x)) {
                        let moved: Vec<CellRange> = rs
                            .iter()
                            .filter_map(|r| match sh.range(r) {
                                Moved::Same | Moved::Split => Some(*r),
                                Moved::To(to) => Some(to),
                                Moved::Gone => None,
                            })
                            .collect();
                        if !moved.is_empty() && moved != rs {
                            c.set("sqref", &write_sqref(&moved));
                        }
                    }
                }
            }
        }
    }
    move_parts(book, i, sh, out)?;
    Ok(())
}

/// Whether a row holds nothing worth keeping once its cells are gone.
fn is_plain(row: &Row) -> bool {
    row.cells.is_empty()
        && row.extra.is_empty()
        && row.attrs.iter().all(|a| matches!(a.0.as_str(), "r" | "spans" | "x14ac:dyDescent"))
}

fn move_cells(book: &mut Book, i: usize, sh: &RowShift, out: &mut Vec<Notice>) -> Result<(), String> {
    let (c0, c1) = sh.cols();
    let from = sh.from_row();
    let name = book.sheets[i].name.clone();
    let mut removed_strings = 0i64;
    let st = book.store_mut(i)?;
    let rows = st.take_rows(from, MAX_ROW);
    let mut map: BTreeMap<u32, Row> = BTreeMap::new();
    let mut moved: Vec<(u32, Cell)> = vec![];
    let mut had: BTreeSet<u32> = BTreeSet::new();
    for mut row in rows {
        had.insert(row.r);
        let (inside, outside): (Vec<Cell>, Vec<Cell>) = row.cells.drain(..).partition(|c| (c0..=c1).contains(&c.col));
        row.cells = outside;
        for c in inside {
            match sh.row(c.col, row.r) {
                Some(nr) => moved.push((nr, c)),
                None => {
                    if c.ty() == "s" {
                        removed_strings += 1;
                    }
                    if !c.is_blank() && c.f.is_none() {
                        // a deleted value: nothing to report beyond the operation itself
                    }
                }
            }
        }
        map.insert(row.r, row);
    }
    for (nr, mut c) in moved {
        c.set_row(nr);
        let row = map.entry(nr).or_insert_with(|| Row::new(nr));
        match row.cells.binary_search_by_key(&c.col, |x| x.col) {
            Ok(k) => row.cells[k] = c,
            Err(k) => row.cells.insert(k, c),
        }
    }
    let mut back = vec![];
    for (_, mut row) in map {
        if is_plain(&row) {
            continue;
        }
        row.fix_spans();
        back.push(row);
    }
    st.put_rows(back);
    let _ = (had, name, out);
    if removed_strings > 0 {
        if let Some(s) = book.sst.as_mut() {
            s.refs_delta -= removed_strings;
        }
    }
    Ok(())
}

/// Anchors outside the worksheet part: drawings, notes, charts' data, pivot caches, the calc chain.
fn move_parts(book: &mut Book, i: usize, sh: &RowShift, out: &mut Vec<Notice>) -> Result<(), String> {
    let name = book.sheets[i].name.clone();
    let part = book.sheets[i].part.clone();
    let rels = book.rels_of(&part);
    let mut edits: Vec<(String, Vec<u8>)> = vec![];
    for r in rels.iter().filter(|r| !r.external) {
        let target = opc::resolve_target(&part, &r.target);
        let Some(data) = package::get(&book.parts, &target) else { continue };
        match r.short_type() {
            "drawing" => {
                let Ok(mut d) = xml::parse(data) else { continue };
                let mut changed = false;
                for a in d.root.elements_mut() {
                    let (c0, c1) = sh.cols();
                    let from_col = a.elements().find(|e| e.local() == "from").and_then(|f| num(f, "col"));
                    if !from_col.is_some_and(|c| (c0..=c1).contains(&c)) {
                        continue;
                    }
                    for end in ["from", "to"] {
                        if let Some(e) = a.elements_mut().find(|e| e.local() == end) {
                            if let (Some(col), Some(row)) = (num(e, "col"), num(e, "row")) {
                                let new = match sh.row(col.max(c0).min(c1), row + 1) {
                                    Some(nr) => nr - 1,
                                    None => sh.from_row() - 1,
                                };
                                if new != row {
                                    set_num(e, "row", new);
                                    changed = true;
                                }
                            }
                        }
                    }
                }
                if changed {
                    edits.push((target.clone(), xml::write_doc(&d)));
                }
                // Charts of the drawing read the sheet's cells.
                for cr in book.rels_of(&target).iter().filter(|x| x.short_type() == "chart" && !x.external) {
                    let cp = opc::resolve_target(&target, &cr.target);
                    if let Some(new) = shift_chart(book, &cp, &name, sh) {
                        edits.push((cp, new));
                    }
                }
            }
            "comments" => {
                let Ok(mut d) = xml::parse(data) else { continue };
                let mut changed = false;
                if let Some(list) = d.root.elements_mut().find(|e| e.local() == "commentList") {
                    list.children.retain(|n| {
                        let Node::El(c) = n else { return true };
                        let Some(cr) = c.get("ref").and_then(|x| CellRef::parse(&x)) else { return true };
                        if sh.row(cr.col, cr.row).is_none() {
                            notice(out, "removed", format!("{name}!{cr}"), "the note went with its deleted row");
                            changed = true;
                            return false;
                        }
                        true
                    });
                    for c in list.elements_mut() {
                        if let Some(cr) = c.get("ref").and_then(|x| CellRef::parse(&x)) {
                            if let Some(nr) = sh.row(cr.col, cr.row) {
                                if nr != cr.row {
                                    c.set("ref", &CellRef::new(cr.col, nr).to_string());
                                    changed = true;
                                }
                            }
                        }
                    }
                }
                if changed {
                    edits.push((target.clone(), xml::write_doc(&d)));
                }
            }
            "threadedComment" => {
                let Ok(mut d) = xml::parse(data) else { continue };
                let mut changed = false;
                for c in d.root.elements_mut() {
                    if let Some(cr) = c.get("ref").and_then(|x| CellRef::parse(&x)) {
                        if let Some(nr) = sh.row(cr.col, cr.row) {
                            if nr != cr.row {
                                c.set("ref", &CellRef::new(cr.col, nr).to_string());
                                changed = true;
                            }
                        }
                    }
                }
                if changed {
                    edits.push((target.clone(), xml::write_doc(&d)));
                }
            }
            "vmlDrawing" => {
                // Legacy note boxes: <x:Row> and <x:Column> (0-based) in each shape's client data.
                let text = String::from_utf8_lossy(data).into_owned();
                if let Some(new) = shift_vml(&text, sh) {
                    edits.push((target.clone(), new.into_bytes()));
                }
            }
            "pivotTable" => {
                let Ok(mut d) = xml::parse(data) else { continue };
                let mut changed = false;
                if let Some(loc) = d.root.elements_mut().find(|e| e.local() == "location") {
                    if let Some(rr) = loc.get("ref").and_then(|x| CellRange::parse(&x)) {
                        if let Moved::To(to) = sh.range(&rr) {
                            loc.set("ref", &to.to_string());
                            changed = true;
                        }
                    }
                }
                if changed {
                    edits.push((target.clone(), xml::write_doc(&d)));
                }
            }
            _ => {}
        }
    }
    // Pivot caches whose source is on the sheet: their source moves, and they refresh when opened.
    let wb = book.wb_part.clone();
    for r in book.rels_of(&wb).iter().filter(|r| r.short_type() == "pivotCacheDefinition" && !r.external) {
        let cp = opc::resolve_target(&wb, &r.target);
        let Some(mut d) = package::get(&book.parts, &cp).and_then(|d| xml::parse(d).ok()) else { continue };
        let mut changed = false;
        d.root.walk_mut(&mut |e| {
            if e.local() == "worksheetSource" && e.get("sheet").is_some_and(|s| s.eq_ignore_ascii_case(&name)) {
                if let Some(rr) = e.get("ref").and_then(|x| CellRange::parse(&x)) {
                    match sh.range(&rr) {
                        Moved::To(to) => {
                            e.set("ref", &to.to_string());
                            changed = true;
                        }
                        Moved::Gone => {
                            changed = true;
                        }
                        _ => {}
                    }
                }
            }
        });
        if changed {
            d.root.set("refreshOnLoad", "1");
            notice(
                out,
                "pivot-refresh",
                cp.clone(),
                "the pivot table's source moved; it refreshes when the file is opened",
            );
            edits.push((cp, xml::write_doc(&d)));
        }
    }
    for (p, data) in edits {
        if let Some(x) = book.parts.iter_mut().find(|x| x.name == p) {
            x.data = data;
        }
    }
    drop_calc_chain(book);
    Ok(())
}

fn num(e: &Element, local: &str) -> Option<u32> {
    e.elements().find(|x| x.local() == local).and_then(|x| x.text_of(&[x.name.as_str()]).trim().parse().ok())
}

fn set_num(e: &mut Element, local: &str, v: u32) {
    if let Some(x) = e.elements_mut().find(|x| x.local() == local) {
        x.children = vec![Node::Text(v.to_string())];
    }
}

fn shift_vml(text: &str, sh: &RowShift) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut changed = false;
    // Each x:ClientData block with x:Row and x:Column.
    while let Some(k) = rest.find("<x:ClientData") {
        let Some(end) = rest[k..].find("</x:ClientData>").map(|e| k + e) else { break };
        let block = &rest[k..end];
        let get = |tag: &str| {
            block.find(&format!("<x:{tag}>")).and_then(|a| {
                let s = a + tag.len() + 4;
                block[s..].find('<').map(|b| (s, s + b, block[s..s + b].trim().parse::<u32>().ok()))
            })
        };
        let mut new_block = block.to_string();
        if let (Some((rs, re, Some(row))), Some((_, _, Some(col)))) = (get("Row"), get("Column")) {
            let nr = match sh.row(col, row + 1) {
                Some(nr) => nr - 1,
                None => row,
            };
            if nr != row {
                new_block = format!("{}{nr}{}", &block[..rs], &block[re..]);
                changed = true;
            }
        }
        out.push_str(&rest[..k]);
        out.push_str(&new_block);
        rest = &rest[end..];
    }
    out.push_str(rest);
    changed.then_some(out)
}

/// A chart's data references into sheet `name`, moved.
fn shift_chart(book: &Book, part: &str, name: &str, sh: &RowShift) -> Option<Vec<u8>> {
    let mut d = xml::parse(package::get(&book.parts, part)?).ok()?;
    let mut changed = false;
    d.root.walk_mut(&mut |e| {
        if e.local() == "f" {
            let t = e.text_of(&[e.name.as_str()]);
            if let Some(new) = shift_formula(&t, "", name, sh) {
                e.children = vec![Node::Text(xml::escape_text(&new))];
                changed = true;
            }
        }
    });
    changed.then(|| xml::write_doc(&d))
}

/// Formula cells moved or changed: the calc chain is stale; Excel rebuilds it.
pub fn drop_calc_chain(book: &mut Book) {
    let wb = book.wb_part.clone();
    let rels = book.rels_of(&wb);
    if let Some(r) = rels.iter().find(|r| r.short_type() == "calcChain") {
        let target = opc::resolve_target(&wb, &r.target);
        let kept: Vec<_> = rels.iter().filter(|x| x.id != r.id).cloned().collect();
        book.set_rels(&wb, kept);
        book.remove_part(&target);
    }
}

/// Is the formula reference token pointing into rows `a..=b` of columns `c0..=c1` of `sheet`?
pub fn refers_into(f: &str, own: &str, sheet: &str, area: &CellRange) -> bool {
    formula::tokenize(f).iter().any(|t| match &t.kind {
        Tok::Ref(r) => {
            let s = r.sheet.as_ref().map_or(own, |p| p.name.as_str());
            s.eq_ignore_ascii_case(sheet) && ref_area(r).is_some_and(|x| x.intersects(area))
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulas_follow_their_rows() {
        let ins = RowShift::Insert { cols: (0, 5), at: 10, n: 2 };
        assert_eq!(shift_formula("SUM(D2:D20)", "매출", "매출", &ins).as_deref(), Some("SUM(D2:D22)"));
        assert_eq!(shift_formula("SUM(D2:D9)", "매출", "매출", &ins), None);
        assert_eq!(shift_formula("$D$12*2", "요약", "매출", &ins), None);
        assert_eq!(shift_formula("매출!$D$12*2", "요약", "매출", &ins).as_deref(), Some("매출!$D$14*2"));
        assert_eq!(shift_formula("'매출'!D12", "요약", "매출", &ins).as_deref(), Some("'매출'!D14"));
        assert_eq!(shift_formula("H12+D12", "매출", "매출", &ins).as_deref(), Some("H12+D14"));
        let del = RowShift::Delete { cols: (0, 5), first: 10, last: 12 };
        assert_eq!(shift_formula("D11+D13", "s", "s", &del).as_deref(), Some("#REF!+D10"));
        assert_eq!(shift_formula("SUM(D2:D20)", "s", "s", &del).as_deref(), Some("SUM(D2:D17)"));
        assert_eq!(shift_formula("Sales[매출]", "s", "s", &del), None);
    }

    #[test]
    fn copies_move_relative_references() {
        assert_eq!(translate("A2*2+$B$1+B$1+Sheet2!C3", 3, 1), "B5*2+$B$1+C$1+Sheet2!D6");
        assert_eq!(translate("A1", -1, 0), "#REF!");
        assert_eq!(translate("SUM(A:A)", 5, 0), "SUM(A:A)");
    }

    #[test]
    fn note_boxes_move() {
        let v = "<x:ClientData ObjectType=\"Note\"><x:Row>11</x:Row><x:Column>1</x:Column></x:ClientData>";
        let ins = RowShift::Insert { cols: (0, 5), at: 10, n: 2 };
        assert_eq!(shift_vml(v, &ins).unwrap(), v.replace(">11<", ">13<"));
    }
}
