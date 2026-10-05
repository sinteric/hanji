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
use crate::store::{has_formula, holds_any_case, sheet_needle, Cell, Row};

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
    let top_first = b.is_none_or(|b| a.row <= b.row);
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
/// The formula cells that `sh` on sheet `target` rewrites: (sheet, row,
/// column, new text if its references move, new `ref` if its span moves).
type Rewrite = (usize, u32, u32, Option<String>, Option<String>);

fn formulas_to_rewrite(book: &mut Book, target: &str, i: usize, sh: &RowShift) -> Result<Vec<Rewrite>, String> {
    let mut out = vec![];
    let needle = sheet_needle(target);
    for k in 0..book.sheets.len() {
        if book.sheets[k].kind != SheetKind::Work {
            continue;
        }
        book.load_store(k)?;
        let own_name = book.sheets[k].name.clone();
        let own = own_name.eq_ignore_ascii_case(target);
        let pre = |b: &[u8]| has_formula(b) && (own || holds_any_case(b, &needle));
        book.store(k).for_each_cell_if(&pre, &mut |r, c| {
            let Some(f) = &c.f else { return };
            let text = c.formula().and_then(|t| shift_formula(&t, &own_name, target, sh));
            let span = (k == i).then(|| f.get("ref").and_then(|x| CellRange::parse(&x))).flatten().and_then(|rr| {
                match sh.range(&rr) {
                    Moved::To(to) => Some(to.to_string()),
                    _ => None,
                }
            });
            if text.is_some() || span.is_some() {
                out.push((k, r, c.col, text, span));
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
    if let RowShift::Insert { at, n, .. } = *sh {
        // Only the populated tail can cross the limit. Do not materialize the
        // empty rows between a small table and a distant ordinary cell.
        let first = at.max(MAX_ROW.saturating_sub(n).saturating_add(1));
        for r in st.row_numbers().filter(|r| *r >= first) {
            if let Some(row) = st.try_row(r)? {
                if let Some(c) = row.cells.iter().find(|c| (c0..=c1).contains(&c.col)) {
                    return Err(format!(
                        "inserting {n} rows would move cell {} on sheet {name} past the worksheet row limit {MAX_ROW}",
                        CellRef::new(c.col, r)
                    ));
                }
            }
        }
    }
    let mut err = None;
    st.for_each_cell_if(&has_formula, &mut |r, c| {
        let Some(f) = &c.f else { return };
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
            _ => {}
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
    // A cross-sheet chart source cannot be represented by one rectangular
    // reference if only some of its columns move. Refuse before mutating.
    for part in chart_parts(book) {
        let Some(d) = package::get(&book.parts, &part).and_then(|data| xml::parse(data).ok()) else { continue };
        let mut split = None;
        d.root.walk(&mut |e| {
            if e.local() != "f" {
                return;
            }
            let f = e.text_of(&[e.name.as_str()]);
            for t in formula::tokenize(&f) {
                let Tok::Ref(r) = &t.kind else { continue };
                let Some(s) = &r.sheet else { continue };
                if !s.external
                    && s.to.is_none()
                    && s.name.eq_ignore_ascii_case(&name)
                    && ref_area(r).is_some_and(|area| sh.range(&area) == Moved::Split)
                {
                    split = Some(f.clone());
                }
            }
        });
        if let Some(f) = split {
            return Err(format!("chart {part} source {f} would be split by rows that move"));
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
/// Move rows; returns the formula cells whose text changed (where they now are).
pub fn apply(
    book: &mut Book,
    i: usize,
    sh: &RowShift,
    out: &mut Vec<Notice>,
) -> Result<Vec<(usize, u32, u32)>, String> {
    let name = book.sheets[i].name.clone();
    // Shared formulas that read the sheet become plain ones first.
    for k in 0..book.sheets.len() {
        if book.sheets[k].kind == SheetKind::Work {
            let reads = k == i || {
                book.load_store(k)?;
                let mut hit = false;
                let needle = sheet_needle(&name);
                book.store(k).for_each_cell_if(&|b| has_formula(b) && holds_any_case(b, &needle), &mut |_, c| {
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
    let mut rewritten = vec![];
    for (k, r, col, text, span) in formulas_to_rewrite(book, &name, i, sh)? {
        let st = book.store_mut(k)?;
        let Some(f) = st.row_mut(r).cells.iter_mut().find(|c| c.col == col).and_then(|c| c.f.as_mut()) else {
            continue;
        };
        if let Some(new) = text {
            f.children = vec![Node::Text(xml::escape_text(&new))];
        }
        if let Some(to) = span {
            f.set("ref", &to);
        }
        let at = if k == i { sh.row(col, r) } else { Some(r) };
        rewritten.extend(at.map(|r| (k, r, col)));
    }
    move_cells(book, i, sh)?;
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
    Ok(rewritten)
}

/// Whether a row holds nothing worth keeping once its cells are gone.
/// A row with nothing of its own: no cells, and no attributes but its number and hints.
pub(crate) fn is_plain(row: &Row) -> bool {
    row.cells.is_empty()
        && row.extra.is_empty()
        && row.attrs.iter().all(|a| matches!(a.0.as_str(), "r" | "spans" | "x14ac:dyDescent"))
}

/// Move the cells of the shifted columns, one row at a time: a row's other
/// cells and its attributes stay at its number, and a row left with nothing
/// of its own goes. Rows are written back as they are complete.
fn move_cells(book: &mut Book, i: usize, sh: &RowShift) -> Result<(), String> {
    let (c0, c1) = sh.cols();
    // Rows below `r - lag` get nothing from rows after `r`.
    let lag = match *sh {
        RowShift::Delete { first, last, .. } => last - first + 1,
        RowShift::Insert { .. } => 0,
    };
    let mut removed_strings = 0i64;
    let st = book.store_mut(i)?;
    let tail = st.take_tail(sh.from_row());
    let mut pending: BTreeMap<u32, Row> = BTreeMap::new();
    let flush = |pending: &mut BTreeMap<u32, Row>, below: u32, st: &mut crate::store::Store| {
        while let Some(entry) = pending.first_entry().filter(|e| *e.key() < below) {
            let mut row = entry.remove();
            if !is_plain(&row) {
                row.fix_spans();
                st.push_row(&row);
            }
        }
    };
    for row in tail {
        let mut row = row?;
        let r = row.r;
        flush(&mut pending, r.saturating_sub(lag), st);
        let (inside, outside): (Vec<Cell>, Vec<Cell>) = row.cells.drain(..).partition(|c| (c0..=c1).contains(&c.col));
        row.cells = outside;
        // Cells moved here earlier (an insert) join the row's own.
        if let Some(early) = pending.remove(&r) {
            for c in early.cells {
                put_cell(&mut row, c);
            }
        }
        pending.insert(r, row);
        for mut c in inside {
            match sh.row(c.col, r) {
                Some(nr) => {
                    c.set_row(nr);
                    put_cell(pending.entry(nr).or_insert_with(|| Row::new(nr)), c);
                }
                None if c.ty() == "s" => removed_strings += 1,
                None => {}
            }
        }
    }
    flush(&mut pending, u32::MAX, st);
    if removed_strings > 0 {
        if let Some(s) = book.sst.as_mut() {
            s.refs_delta -= removed_strings;
        }
    }
    Ok(())
}

fn put_cell(row: &mut Row, c: Cell) {
    match row.cells.binary_search_by_key(&c.col, |x| x.col) {
        Ok(k) => row.cells[k] = c,
        Err(k) => row.cells.insert(k, c),
    }
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
    // A chart's source may be on any sheet, independently of its anchor.
    // Visit each related chart once, including charts on chart sheets.
    for cp in chart_parts(book) {
        if let Some(new) = shift_chart(book, &cp, &name, sh) {
            edits.push((cp, new));
        }
    }
    // Pivot caches whose source is a range on the sheet: the range moves
    // (`refresh_pivots` then marks them to refresh when opened).
    let wb = book.wb_part.clone();
    for r in book.rels_of(&wb).iter().filter(|r| r.short_type() == "pivotCacheDefinition" && !r.external) {
        let cp = opc::resolve_target(&wb, &r.target);
        let Some(mut d) = package::get(&book.parts, &cp).and_then(|d| xml::parse(d).ok()) else { continue };
        let mut changed = false;
        d.root.walk_mut(&mut |e| {
            if e.local() == "worksheetSource" && e.get("sheet").is_some_and(|s| s.eq_ignore_ascii_case(&name)) {
                if let Some(Moved::To(to)) = e.get("ref").and_then(|x| CellRange::parse(&x)).map(|rr| sh.range(&rr)) {
                    e.set("ref", &to.to_string());
                    changed = true;
                }
            }
        });
        if changed {
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

/// Chart relationships anywhere in the package, independent of worksheet ownership.
fn chart_parts(book: &Book) -> BTreeSet<String> {
    let mut charts = BTreeSet::new();
    for source in book.parts.iter().filter_map(|p| opc::source_of_rels(&p.name)) {
        for r in book.rels_of(&source).iter().filter(|r| r.short_type() == "chart" && !r.external) {
            charts.insert(opc::resolve_target(&source, &r.target));
        }
    }
    charts
}

/// A chart's data references into sheet `name`, moved.
fn shift_chart(book: &Book, part: &str, name: &str, sh: &RowShift) -> Option<Vec<u8>> {
    let mut d = xml::parse(package::get(&book.parts, part)?).ok()?;
    let mut changed = false;
    d.root.walk_mut(&mut |e| {
        let mut moved = false;
        for f in e.elements_mut().filter(|f| f.local() == "f") {
            let t = f.text_of(&[f.name.as_str()]);
            if let Some(new) = shift_formula(&t, "", name, sh) {
                f.children = vec![Node::Text(xml::escape_text(&new))];
                moved = true;
            }
        }
        if moved {
            // These optional caches describe the old source range. Let the
            // spreadsheet application rebuild them from the rewritten formula.
            e.children.retain(
                |n| !matches!(n, Node::El(c) if matches!(c.local(), "numCache" | "strCache" | "multiLvlStrCache")),
            );
            changed = true;
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

/// Marks the pivot caches whose source the operations changed (a table whose
/// rows or columns changed, or a range holding a written cell) to refresh
/// when the file is opened: their stored records and the pivot tables' cells
/// are otherwise stale.
pub fn refresh_pivots(book: &mut Book, changed: &crate::calc::Changed, out: &mut Vec<Notice>) {
    let wb = book.wb_part.clone();
    let sheet_of = |name: &str| book.sheets.iter().position(|s| s.name.eq_ignore_ascii_case(name));
    let mut edits = vec![];
    for r in book.rels_of(&wb).iter().filter(|r| r.short_type() == "pivotCacheDefinition" && !r.external) {
        let cp = opc::resolve_target(&wb, &r.target);
        let Some(mut d) = package::get(&book.parts, &cp).and_then(|d| xml::parse(d).ok()) else { continue };
        let mut stale = false;
        d.root.walk_mut(&mut |e| {
            if e.local() != "worksheetSource" {
                return;
            }
            if let Some(n) = e.get("name") {
                stale |= changed.tables.iter().any(|t| t.eq_ignore_ascii_case(&n));
            }
            let (Some(i), Some(rr)) =
                (e.get("sheet").and_then(|s| sheet_of(&s)), e.get("ref").and_then(|x| CellRange::parse(&x)))
            else {
                return;
            };
            let inside = |row: u32, col: u32| {
                (rr.first.row..=rr.last.row).contains(&row) && (rr.first.col..=rr.last.col).contains(&col)
            };
            stale |= changed.cells.iter().chain(&changed.formulas).any(|&(k, row, col)| k == i && inside(row, col))
                || changed.areas.iter().chain(&changed.moved).any(|(k, a)| {
                    *k == i
                        && a.first.row <= rr.last.row
                        && rr.first.row <= a.last.row
                        && a.first.col <= rr.last.col
                        && rr.first.col <= a.last.col
                });
        });
        if stale && d.root.get("refreshOnLoad").as_deref() != Some("1") {
            d.root.set("refreshOnLoad", "1");
            notice(
                out,
                "pivot-refresh",
                cp.clone(),
                "the pivot table's source changed; it refreshes when the file is opened",
            );
            edits.push((cp, xml::write_doc(&d)));
        }
    }
    for (p, data) in edits {
        if let Some(x) = book.parts.iter_mut().find(|x| x.name == p) {
            x.data = data;
        }
    }
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
