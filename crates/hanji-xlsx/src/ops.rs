//! Range operations on the workbook (§5.4, §6 round 4 write shape A):
//! `set`, `append_rows`, `insert_rows`, `delete_rows`, `fill_formula`,
//! `set_type`, `add_column`, `sort`, `add_table`, `add_sheet`, applied in
//! order, all or nothing. Values are written into shared strings (or inline
//! strings when the file has no table), numbers, dates and formulas; table
//! ranges, the sheet dimension and everything anchored to moving rows follow
//! (`shift.rs`); formulas whose inputs changed get their cached values
//! (`calc.rs`). Untouched cells keep their bytes; new cells take their
//! column's style. §8: text that starts with `=` is text, never a formula;
//! a formula is written only through a formula column, with structured
//! references and no function that fetches; types come from the column,
//! never from how a value looks.

use std::collections::HashMap;

use hanji_core::cells::{col_letters, parse_sqref, CellRange, CellRef, RowShift, MAX_ROW};
use hanji_core::{notice, EngineError, Kind, Notice, Remainder};
use hanji_format::formula;
use hanji_format::ops::{
    check_ops, date_of, parse_ops, render_errors, ColumnSpec, Formats, RangeOp, Row, SortKey, Value,
};
use hanji_format::sheet::{format_kind, parse_spreadsheet, serialize_spreadsheet, FormatKind};
use hanji_package::xml::{self, Element, Node};

use crate::book::{Book, SheetInfo, SheetKind, TableCol, TableInfo, CT_SHEET, MAIN_NS, R_NS};
use crate::calc::{self, Changed, Recalc};
use crate::numfmt;
use crate::shift::{self, f_element};
use crate::store::{has_formula, holds, Cell, SHEET_ORDER};
use crate::value::CellValue;
use crate::{book_of, model, remainder_of, structure, Applied, XlsxEngine};

/// Number formats round 4 lists, allowed in every workbook besides its own and the built-in ones.
pub const ROUND4_FORMATS: &[&str] =
    &["#,##0", "#,##0.0", "0", "0.0", "0.0%", "0000", "00000", "000000", "yyyy-mm", "yyyy-mm-dd", "@"];

/// What happened to one range entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryOutcome {
    pub id: u64,
    /// `mergeCell`, `conditionalFormatting`, `dataValidation`, `hyperlink`.
    pub tag: String,
    pub sheet: String,
    pub before: String,
    /// Its range now; `None` when it went (the reason is in `notices`).
    pub after: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct OpReport {
    /// Operations applied (structure edits in the text first).
    pub applied: usize,
    /// Every range entry the revision had: where it is now, or gone.
    pub entries: Vec<EntryOutcome>,
    /// What moved or went outside range entries, and warnings (`kind`:
    /// `removed`, `formula-text`, `unchecked-validation`, `pivot-refresh`, …).
    pub notices: Vec<Notice>,
    pub recalc: Recalc,
}

/// A table's columns as the operations see them: name, type, format, formula.
type Cols = Vec<(String, String, String, String)>;

struct Ctx {
    changed: Changed,
    notices: Vec<Notice>,
    /// Column declarations per table (lower-case name), kept current as operations change them.
    cols: HashMap<String, Cols>,
    /// Multi-cell array formulas per sheet, as found (cleared when rows move).
    arrays: HashMap<usize, Vec<CellRange>>,
}

fn refused(m: String) -> EngineError {
    EngineError::Refused(m)
}

pub fn apply(text: &str, rem: &Remainder, ops_json: &str) -> Result<Applied, EngineError> {
    let parsed = parse_spreadsheet(text, &XlsxEngine::names(rem)).map_err(EngineError::Invalid)?;
    let mut book = book_of(rem)?;
    let template = parsed.front.template.as_deref();
    let current = model::structure(&mut book, template).map_err(EngineError::Package)?;
    let (text_ops, drops) = structure::reconcile(&parsed, &current).map_err(refused)?;
    let user_ops =
        parse_ops(ops_json).map_err(|e| refused(format!("the operations are not valid:\n{}", render_errors(&e))))?;
    let mut formats: Vec<String> = book.styles.all_codes();
    for f in ROUND4_FORMATS {
        if !formats.iter().any(|x| x == f) {
            formats.push(f.to_string());
        }
    }
    // Column formats the file already uses are allowed too.
    for (_, t) in current.tables() {
        for c in &t.columns {
            if !formats.contains(&c.format) {
                formats.push(c.format.clone());
            }
        }
    }
    let all: Vec<RangeOp> = text_ops.iter().cloned().chain(user_ops.iter().cloned()).collect();
    if let Err(errs) = check_ops(&all, &current, &Formats(formats)) {
        let n = text_ops.len();
        let lines: Vec<String> = errs
            .into_iter()
            .map(|mut e| {
                if e.op > n {
                    e.op -= n;
                    e.to_string()
                } else if e.op > 0 {
                    format!("the structure text: {}", e.message)
                } else {
                    e.to_string()
                }
            })
            .collect();
        return Err(refused(format!("the operations are not valid:\n{}", lines.join("\n"))));
    }
    let before: Vec<(u64, String, usize, String)> = book
        .entries
        .iter()
        .filter(|e| e.kind == Kind::Range)
        .map(|e| (e.id, e.meta.tag.clone(), e.path[0], e.meta.range.clone().unwrap_or_default()))
        .collect();
    let mut cx = Ctx { changed: Changed::default(), notices: vec![], cols: HashMap::new(), arrays: HashMap::new() };
    for (_, t) in current.tables() {
        cx.cols.insert(
            t.name.to_lowercase(),
            t.columns.iter().map(|c| (c.name.clone(), c.ty.clone(), c.format.clone(), c.formula.clone())).collect(),
        );
    }
    for id in &drops {
        drop_keep(&mut book, rem, id, &mut cx).map_err(refused)?;
    }
    let n_text = text_ops.len();
    for (k, op) in all.iter().enumerate() {
        let which = if k < n_text {
            "the structure text".to_string()
        } else {
            format!("operation {} ({})", k - n_text + 1, op.name())
        };
        apply_op(&mut book, op, &mut cx).map_err(|m| refused(format!("{which}: {m}")))?;
    }
    let recalc = calc::recompute(&mut book, &cx.changed).map_err(refused)?;
    shift::refresh_pivots(&mut book, &cx.changed, &mut cx.notices);
    for i in 0..book.sheets.len() {
        if book.is_dirty(i) {
            fix_dimension(&mut book, i).map_err(refused)?;
        }
    }
    let names: Vec<String> = book.sheets.iter().map(|s| s.name.clone()).collect();
    let entries = before
        .into_iter()
        .map(|(id, tag, sheet, range)| EntryOutcome {
            id,
            tag,
            sheet: names[sheet].clone(),
            before: range,
            after: book.entries.iter().find(|e| e.id == id).and_then(|e| e.meta.range.clone()),
        })
        .collect();
    let report = OpReport { applied: all.len(), entries, notices: cx.notices, recalc };
    let text = serialize_spreadsheet(&model::structure(&mut book, template).map_err(EngineError::Package)?);
    let remainder = remainder_of(book, rem.next_id);
    Ok(Applied { text, remainder, report })
}

/// Remove what a deleted placeholder stood for (§2 rule 8: an explicit act).
fn drop_keep(book: &mut Book, rem: &Remainder, id: &str, cx: &mut Ctx) -> Result<(), String> {
    let e = rem
        .entries
        .iter()
        .find(|e| e.meta.keep.as_ref().is_some_and(|k| k.id == id))
        .ok_or(format!("no placeholder {id}"))?;
    let k = e.meta.keep.clone().unwrap();
    let sheet = e.path[0];
    let tag = match k.kind.as_str() {
        "merged-cells" => "mergeCell",
        "conditional-format" => "conditionalFormatting",
        "data-validation" => "dataValidation",
        "hyperlinks" => "hyperlink",
        other => {
            return Err(format!(
                "deleting the {other} placeholder {id} is not supported yet; keep its line <keep id=\"{id}\" …/> (the {other} stays in the file)"
            ))
        }
    };
    let gone: Vec<u64> = book
        .entries
        .iter()
        .filter(|x| x.kind == Kind::Range && x.path[0] == sheet && x.meta.tag == tag)
        .map(|x| x.id)
        .collect();
    if tag == "hyperlink" {
        // The hyperlinks' relationships go with them.
        let ids: Vec<String> = book
            .entries
            .iter()
            .filter(|x| gone.contains(&x.id))
            .filter_map(|x| {
                xml::fragment(&x.xml[0]).attrs.iter().find(|a| a.0.ends_with(":id")).map(|a| xml::unescape(&a.1))
            })
            .collect();
        let part = book.sheets[sheet].part.clone();
        let rels: Vec<_> = book.rels_of(&part).into_iter().filter(|r| !ids.contains(&r.id)).collect();
        book.set_rels(&part, rels);
    }
    book.entries.retain(|x| !gone.contains(&x.id));
    book.store_mut(sheet)?;
    notice(
        &mut cx.notices,
        "removed",
        format!("{} {}", book.sheets[sheet].name, k.kind),
        format!("the placeholder was deleted: {}", k.summary),
    );
    Ok(())
}

fn sheet_of(book: &Book, name: &str) -> Result<usize, String> {
    book.sheet_index(name).ok_or_else(|| format!("there is no sheet \"{name}\""))
}

fn table_of(book: &Book, name: &str) -> Result<usize, String> {
    book.table_index(name).ok_or_else(|| format!("there is no table \"{name}\""))
}

fn apply_op(book: &mut Book, op: &RangeOp, cx: &mut Ctx) -> Result<(), String> {
    let table = match op {
        RangeOp::AppendRows { table, .. }
        | RangeOp::InsertRows { table, .. }
        | RangeOp::DeleteRows { table, .. }
        | RangeOp::FillFormula { table, .. }
        | RangeOp::SetType { table, .. }
        | RangeOp::AddColumn { table, .. }
        | RangeOp::Sort { table, .. } => Some(table_of(book, table)?),
        _ => None,
    };
    if let Some(k) = table {
        book.load_store(book.tables[k].sheet)?;
    }
    match op {
        RangeOp::Set { range, values } => op_set(book, range, values, cx),
        RangeOp::AppendRows { table, rows } => {
            let k = table_of(book, table)?;
            let at = book.tables[k].data_rows().1 + 1;
            insert_rows(book, k, at, rows, cx)
        }
        RangeOp::InsertRows { table, before, rows } => {
            let k = table_of(book, table)?;
            insert_rows(book, k, *before, rows, cx)
        }
        RangeOp::DeleteRows { table, first, last } => delete_rows(book, table_of(book, table)?, *first, *last, cx),
        RangeOp::FillFormula { table, column, formula } => {
            fill_formula(book, table_of(book, table)?, column, formula, cx)
        }
        RangeOp::SetType { table, column, ty, format } => {
            set_type(book, table_of(book, table)?, column, ty, format.as_deref(), cx)
        }
        RangeOp::AddColumn { table, column } => add_column(book, table_of(book, table)?, column, cx),
        RangeOp::Sort { table, keys } => sort(book, table_of(book, table)?, keys, cx),
        RangeOp::AddTable { sheet, name, anchor, columns, rows } => {
            add_table(book, sheet, name, anchor, columns, rows, cx)
        }
        RangeOp::AddSheet { name } => add_sheet(book, name),
    }
}

/// What a cell becomes.
enum W {
    Clear,
    Num(f64),
    Str(String),
    Bool(bool),
}

/// The write for `v` into a column of type `ty` (`None`: outside tables,
/// typed by the cell's own format).
fn to_write(book: &Book, v: &Value, ty: Option<&str>, fmt: &str, at: &str) -> Result<W, String> {
    let date = |s: &str| date_of(s).map(|(y, m, d)| numfmt::serial_of(y as i64, m, d, book.date1904));
    Ok(match (ty, v) {
        (_, Value::Null) => W::Clear,
        (Some("date"), Value::Text(s)) => {
            W::Num(date(s).ok_or_else(|| format!("{at}: {s:?} is not a date \"YYYY-MM-DD\""))?)
        }
        (Some("text"), Value::Text(s)) => W::Str(s.clone()),
        (Some("number"), Value::Number(n)) => W::Num(*n),
        (Some("mixed") | None, Value::Text(s))
            if ty.is_none() && format_kind(fmt) == FormatKind::Date && date(s).is_some() =>
        {
            W::Num(date(s).unwrap())
        }
        (Some("mixed") | None, Value::Text(s)) => W::Str(s.clone()),
        (Some("mixed") | None, Value::Number(n)) => W::Num(*n),
        (Some("mixed") | None, Value::Bool(b)) => W::Bool(*b),
        (Some(t), v) => return Err(format!("{at}: {v} does not fit a {t} column")),
    })
}

/// Write `w` into cell (`col`, `row`) of sheet `i`, its style `style` when given.
fn write_cell(
    book: &mut Book,
    i: usize,
    row: u32,
    col: u32,
    w: W,
    style: Option<u32>,
    cx: &mut Ctx,
) -> Result<(), String> {
    let at = format!("{}!{}{row}", book.sheets[i].name, col_letters(col));
    let sst_index = match &w {
        W::Str(s) => {
            if s.starts_with('=') {
                notice(
                    &mut cx.notices,
                    "formula-text",
                    at.clone(),
                    format!("{s:?} is stored as text, never as a formula (§8); a formula belongs to a formula column"),
                );
            }
            Some(book.sst_mut().intern(s))
        }
        _ => None,
    };
    prepare_cell(book, i, row, col, &at, cx)?;
    let st = book.store_mut(i)?;
    let cell = st.row_mut(row).cell_mut(col);
    let was_string = cell.ty() == "s";
    cell.clear_value();
    if let Some(s) = style {
        if s == 0 {
            cell.remove("s");
        } else {
            cell.set("s", &s.to_string());
        }
    }
    match w {
        W::Clear => {}
        W::Num(n) => cell.v = Some(calc::num_text(n)),
        W::Str(_) => {
            cell.set("t", "s");
            cell.v = Some(sst_index.unwrap().to_string());
        }
        W::Bool(b) => {
            cell.set("t", "b");
            cell.v = Some(if b { "1" } else { "0" }.into());
        }
    }
    let delta = i64::from(cell.ty() == "s") - i64::from(was_string);
    if delta != 0 {
        book.sst_mut().refs_delta += delta;
    }
    cx.changed.cells.insert((i, row, col));
    Ok(())
}

/// A formula cell for a formula column (`text` as the file stores it, no `=`).
fn write_formula(
    book: &mut Book,
    i: usize,
    row: u32,
    col: u32,
    text: &str,
    style: Option<u32>,
    cx: &mut Ctx,
) -> Result<(), String> {
    prepare_cell(book, i, row, col, &format!("{}!{}{row}", book.sheets[i].name, col_letters(col)), cx)?;
    let st = book.store_mut(i)?;
    let prefix = st.prefix.clone();
    let cell = st.row_mut(row).cell_mut(col);
    let was_string = cell.ty() == "s";
    cell.clear_value();
    if let Some(s) = style.filter(|s| *s != 0) {
        cell.set("s", &s.to_string());
    }
    cell.f = Some(f_element(&prefix, text));
    if was_string {
        book.sst_mut().refs_delta -= 1;
    }
    cx.changed.formulas.insert((i, row, col));
    Ok(())
}

/// Before a cell is written: refused when it is part of an array formula or
/// a data table (the engine does not model them); the sheet's shared
/// formulas become plain ones when the cell holds one (a shared formula's
/// other cells read their text from its first cell).
fn prepare_cell(book: &mut Book, i: usize, row: u32, col: u32, at: &str, cx: &mut Ctx) -> Result<(), String> {
    if let std::collections::hash_map::Entry::Vacant(slot) = cx.arrays.entry(i) {
        let mut found = vec![];
        book.store(i).for_each_cell_if(
            &|b| has_formula(b) && (holds(b, b"array") || holds(b, b"dataTable")),
            &mut |r, c| {
                if let Some(f) =
                    c.f.as_ref().filter(|f| matches!(f.get("t").as_deref(), Some("array") | Some("dataTable")))
                {
                    found.push(
                        f.get("ref")
                            .and_then(|x| CellRange::parse(&x))
                            .unwrap_or(CellRange::cell(CellRef::new(c.col, r))),
                    );
                }
            },
        );
        slot.insert(found);
    }
    if let Some(a) = cx.arrays[&i].iter().find(|a| a.contains(CellRef::new(col, row))) {
        return Err(format!("{at} is part of the array formula or data table at {a}, which the engine does not model"));
    }
    let shared = book
        .store(i)
        .row(row)
        .and_then(|r| r.cell(col).and_then(|c| c.f.as_ref()).map(|f| f.get("t").as_deref() == Some("shared")));
    if shared == Some(true) {
        shift::unshare(book, i)?;
    }
    Ok(())
}

/// The table holding cell (`col`, `row`) of sheet `i`.
fn table_at(book: &Book, i: usize, col: u32, row: u32) -> Option<usize> {
    book.tables.iter().position(|t| t.sheet == i && t.range.contains(CellRef::new(col, row)))
}

fn op_set(book: &mut Book, range: &str, values: &[Vec<Value>], cx: &mut Ctx) -> Result<(), String> {
    let (sheet, cells) = hanji_format::ops::split_range(range).ok_or("bad range")?;
    let i = sheet_of(book, &sheet)?;
    if book.sheets[i].kind != SheetKind::Work {
        return Err(format!("sheet {sheet} is a chart sheet; it has no cells"));
    }
    let r = CellRange::parse(&cells).ok_or("bad range")?;
    book.load_store(i)?;
    for (dr, row) in values.iter().enumerate() {
        for (dc, v) in row.iter().enumerate() {
            let (col, rr) = (r.first.col + dc as u32, r.first.row + dr as u32);
            let at = format!("{}!{}{rr}", book.sheets[i].name, col_letters(col));
            check_merge(book, i, col, rr, &at)?;
            let (ty, fmt, style) = match table_at(book, i, col, rr) {
                Some(k) => {
                    let t = &book.tables[k];
                    let (d0, d1) = t.data_rows();
                    if rr < d0 || rr > d1 {
                        return Err(format!(
                            "{at} is a header or totals cell of table {}; write its data rows",
                            t.name
                        ));
                    }
                    let ci = (col - t.range.first.col) as usize;
                    let c = &cx.cols[&t.name.to_lowercase()][ci];
                    if !c.3.is_empty() {
                        return Err(format!(
                            "{at} is in {}[{}], a formula column; its cells are computed",
                            t.name, c.0
                        ));
                    }
                    (Some(c.1.clone()), c.2.clone(), None)
                }
                None => {
                    let s = book.store(i).row(rr).and_then(|x| x.cell(col).map(Cell::style)).unwrap_or(0);
                    (None, book.styles.format_of(s), None)
                }
            };
            check_validation(book, i, col, rr, v, &at, cx)?;
            let w = to_write(book, v, ty.as_deref(), &fmt, &at)?;
            write_cell(book, i, rr, col, w, style, cx)?;
        }
    }
    Ok(())
}

fn merges_at(book: &Book, i: usize, col: u32, row: u32) -> Option<CellRange> {
    book.entries
        .iter()
        .filter(|e| e.kind == Kind::Range && e.path[0] == i && e.meta.tag == "mergeCell")
        .filter_map(|e| CellRange::parse(e.meta.range.as_deref()?))
        .find(|m| m.contains(CellRef::new(col, row)))
}

fn check_merge(book: &Book, i: usize, col: u32, row: u32, at: &str) -> Result<(), String> {
    match merges_at(book, i, col, row) {
        Some(m) if m.first != CellRef::new(col, row) => Err(format!(
            "{at} is covered by the merged cells {m}; a merged area's value is in its top-left cell, {}",
            m.first
        )),
        _ => Ok(()),
    }
}

/// A value checked against the data validation of its cell. List rules (a
/// constant list or a range of cells), number, date and text-length rules
/// with constant bounds are checked. Any other rule cannot be: when Excel
/// would refuse a value that breaks it (it shows its error, in the `stop`
/// style) the write is refused; otherwise it is reported unchecked.
fn check_validation(
    book: &mut Book,
    i: usize,
    col: u32,
    row: u32,
    v: &Value,
    at: &str,
    cx: &mut Ctx,
) -> Result<(), String> {
    if *v == Value::Null {
        return Ok(());
    }
    let rules: Vec<(String, String)> = book
        .entries
        .iter()
        .filter(|e| e.kind == Kind::Range && e.path[0] == i && e.meta.tag == "dataValidation")
        .filter(|e| {
            parse_sqref(e.meta.range.as_deref().unwrap_or(""))
                .is_some_and(|rs| rs.iter().any(|r| r.contains(CellRef::new(col, row))))
        })
        .map(|e| (e.xml[0].clone(), e.meta.range.clone().unwrap_or_default()))
        .collect();
    for (rule_xml, range) in rules {
        let x = xml::fragment(&rule_xml);
        let ty = x.get("type").unwrap_or_else(|| "none".into());
        let f = |n: &str| x.elements().find(|c| c.local() == n).map(|c| xml::unescape(&c.text_of(&[c.name.as_str()])));
        let fail = |what: String| Err(format!("{at}: {v} breaks the data validation of {range} ({what})"));
        let text = match v {
            Value::Text(s) => s.clone(),
            Value::Number(n) => numfmt::general(*n),
            Value::Bool(b) => (if *b { "TRUE" } else { "FALSE" }).into(),
            Value::Null => String::new(),
        };
        let bounds = || {
            (
                f("formula1").and_then(|s| s.trim().parse::<f64>().ok()),
                f("formula2").and_then(|s| s.trim().parse::<f64>().ok()),
            )
        };
        let op = x.get("operator").unwrap_or_else(|| "between".into());
        let within = |n: f64, a: Option<f64>, b: Option<f64>| -> Option<bool> {
            Some(match (op.as_str(), a, b) {
                ("between", Some(a), Some(b)) => (a..=b).contains(&n),
                ("notBetween", Some(a), Some(b)) => !(a..=b).contains(&n),
                ("equal", Some(a), _) => n == a,
                ("notEqual", Some(a), _) => n != a,
                ("greaterThan", Some(a), _) => n > a,
                ("lessThan", Some(a), _) => n < a,
                ("greaterThanOrEqual", Some(a), _) => n >= a,
                ("lessThanOrEqual", Some(a), _) => n <= a,
                _ => return None,
            })
        };
        let rule = |a: Option<f64>, b: Option<f64>| {
            format!(
                "{op} {}{}",
                a.map(numfmt::general).unwrap_or_default(),
                b.map(|b| format!(" and {}", numfmt::general(b))).unwrap_or_default()
            )
        };
        // `None`: the rule cannot be checked (why).
        let unchecked: Option<String> = match ty.as_str() {
            "none" => None,
            "list" => match f("formula1") {
                Some(l) if l.starts_with('"') && l.ends_with('"') && l.len() >= 2 => {
                    let items: Vec<&str> = l[1..l.len() - 1].split(',').map(str::trim).collect();
                    if !items.iter().any(|it| it.eq_ignore_ascii_case(text.trim())) {
                        return fail(format!("one of {}", items.join(", ")));
                    }
                    None
                }
                Some(l) => match list_cells(book, i, &l) {
                    Some(items) => {
                        if !items.iter().any(|it| it.eq_ignore_ascii_case(text.trim())) {
                            return fail(format!("a value of {}", l.trim_start_matches('=')));
                        }
                        None
                    }
                    None => Some(format!("its list is the formula {l}")),
                },
                None => Some("it has no list".into()),
            },
            "whole" | "decimal" => {
                let Value::Number(n) = v else { return fail(format!("a {ty} number")) };
                if ty == "whole" && n.fract() != 0.0 {
                    return fail("a whole number".into());
                }
                let (a, b) = bounds();
                match within(*n, a, b) {
                    Some(true) => None,
                    Some(false) => return fail(rule(a, b)),
                    None => Some("its bounds are formulas".into()),
                }
            }
            "date" => {
                let serial = match v {
                    Value::Number(n) => Some(*n),
                    Value::Text(t) => date_of(t).map(|(y, m, d)| numfmt::serial_of(y as i64, m, d, book.date1904)),
                    _ => None,
                };
                let Some(n) = serial else { return fail("a date".into()) };
                let (a, b) = bounds();
                match within(n, a, b) {
                    Some(true) => None,
                    Some(false) => return fail(format!("a date {}", rule(a, b))),
                    None => Some("its bounds are formulas".into()),
                }
            }
            "textLength" => {
                let (a, b) = bounds();
                match within(text.chars().count() as f64, a, b) {
                    Some(true) => None,
                    Some(false) => return fail(format!("a text length {}", rule(a, b))),
                    None => Some("its bounds are formulas".into()),
                }
            }
            other => Some(format!("it is a {other} rule")),
        };
        if let Some(why) = unchecked {
            // Excel refuses a value only when the rule shows its error, in the `stop` style.
            let refuses = x.get("showErrorMessage").is_some_and(|v| v == "1" || v == "true")
                && x.get("errorStyle").is_none_or(|v| v == "stop");
            match refuses {
                false => notice(
                    &mut cx.notices,
                    "unchecked-validation",
                    at.to_string(),
                    format!("the data validation of {range} is not checked: {why}"),
                ),
                true => {
                    return Err(format!(
                        "{at}: the data validation of {range} cannot be checked ({why}), and it refuses values that break it"
                    ))
                }
            }
        }
    }
    Ok(())
}

/// The values of a list validation's range (`$A$1:$A$9`, `Sheet!$A$1:$A$9`), as shown.
fn list_cells(book: &mut Book, i: usize, formula: &str) -> Option<Vec<String>> {
    let toks = formula::tokenize(formula.trim_start_matches('='));
    // One reference and nothing else.
    let mut toks = toks.iter().filter(|t| !matches!(t.kind, formula::Tok::Space));
    let (Some(formula::Token { kind: formula::Tok::Ref(r), .. }), None) = (toks.next(), toks.next()) else {
        return None;
    };
    let (a, b) = (r.first?, r.last.or(r.first)?);
    let sheet = match &r.sheet {
        Some(p) if p.external || p.to.is_some() => return None,
        Some(p) => book.sheets.iter().position(|s| s.name.eq_ignore_ascii_case(&p.name))?,
        None => i,
    };
    let area = CellRange::new(CellRef::new(a.col?, a.row?), CellRef::new(b.col?, b.row?));
    if book.sheets[sheet].kind != SheetKind::Work {
        return None;
    }
    book.load_store(sheet).ok()?;
    let mut out = vec![];
    for row in book.store(sheet).rows_in(area.first.row, area.last.row) {
        for c in row.cells.iter().filter(|c| (area.first.col..=area.last.col).contains(&c.col)) {
            // As shown, and as the value is written (`1` for a `1.00`).
            let shown = book.display(c);
            let value = match book.value(c) {
                CellValue::Number(n) => numfmt::general(n),
                CellValue::Text(t) => t,
                _ => String::new(),
            };
            out.extend([shown, value].into_iter().map(|x| x.trim().to_string()).filter(|x| !x.is_empty()));
        }
    }
    Some(out)
}

/// The style a new cell of table `k`'s column `ci` takes: its neighbour's in
/// the column (the row above, else below), with the column's format.
fn column_style(book: &mut Book, k: usize, ci: usize, near: &[u32], fmt: &str) -> Result<u32, String> {
    let t = &book.tables[k];
    let col = t.range.first.col + ci as u32;
    let st = book.store(t.sheet);
    let mut s = None;
    for &r in near {
        if let Some(c) = st.row(r).and_then(|x| x.cell(col).cloned()) {
            s = Some(c.style());
            break;
        }
    }
    let s = s.unwrap_or(0);
    if fmt.is_empty() || book.styles.format_of(s) == fmt || (fmt == "General" && book.styles.format_of(s) == "General")
    {
        return Ok(s);
    }
    book.styles.with_format(s, fmt)
}

fn cols_of(cx: &Ctx, t: &TableInfo) -> Cols {
    cx.cols.get(&t.name.to_lowercase()).cloned().unwrap_or_default()
}

fn insert_rows(book: &mut Book, k: usize, at: u32, rows: &[Row], cx: &mut Ctx) -> Result<(), String> {
    let n = rows.len() as u32;
    if n == 0 {
        return Ok(());
    }
    let (i, (c0, c1), (d0, d1)) = (book.tables[k].sheet, book.tables[k].cols_span(), book.tables[k].data_rows());
    let sh = RowShift::Insert { cols: (c0, c1), at, n };
    shift::check(book, i, &sh, Some(k))?;
    let old_last = book.tables[k].range.last.row;
    let rewritten = shift::apply(book, i, &sh, &mut cx.notices)?;
    cx.arrays.remove(&i);
    cx.changed.follow(i, &sh);
    cx.changed.formulas.extend(rewritten);
    let t = &mut book.tables[k];
    t.range.last.row = old_last + n;
    t.changed = true;
    let (name, head) = (t.name.clone(), t.range.first.row);
    if at == d1 + 1 {
        // Conditional formats and validations over a whole column of the table grow with it, as in Excel.
        for e in book.entries.iter_mut().filter(|e| e.kind == Kind::Range && e.path[0] == i) {
            if !matches!(e.meta.tag.as_str(), "conditionalFormatting" | "dataValidation") {
                continue;
            }
            let Some(mut rs) = parse_sqref(e.meta.range.as_deref().unwrap_or("")) else { continue };
            let mut grew = false;
            for r in rs.iter_mut() {
                if r.first.col >= c0 && r.last.col <= c1 && r.last.row == d1 && (head..=d0).contains(&r.first.row) {
                    r.last.row += n;
                    grew = true;
                }
            }
            if grew {
                e.meta.range = Some(hanji_core::cells::write_sqref(&rs));
            }
        }
    }
    cx.changed.moved.push((i, CellRange::new(CellRef::new(c0, at), CellRef::new(c1, MAX_ROW))));
    cx.changed.tables.insert(name.clone());
    let cols = cols_of(cx, &book.tables[k]);
    let calc: Vec<Option<String>> = book.tables[k].cols.iter().map(|c| c.calc.clone()).collect();
    // Neighbours for the new cells' style: the data row above, else the one pushed below.
    let near: Vec<u32> = if at > d0 && d1 >= d0 { vec![at - 1] } else { vec![at + n] };
    for (j, row) in rows.iter().enumerate() {
        let r = at + j as u32;
        for (ci, (cname, ty, fmt, _)) in cols.iter().enumerate() {
            let col = c0 + ci as u32;
            let style = column_style(book, k, ci, &near, fmt)?;
            if let Some(f) = &calc[ci] {
                write_formula(book, i, r, col, f, Some(style), cx)?;
                continue;
            }
            let v = row.iter().find(|(c, _)| c == cname).map(|x| &x.1).unwrap_or(&Value::Null);
            let at_s = format!("{}!{}{r}", book.sheets[i].name, col_letters(col));
            check_validation(book, i, col, r, v, &at_s, cx)?;
            let w = to_write(book, v, Some(ty), fmt, &at_s)?;
            if matches!(w, W::Clear) && style == 0 {
                continue;
            }
            write_cell(book, i, r, col, w, Some(style), cx)?;
        }
    }
    Ok(())
}

fn delete_rows(book: &mut Book, k: usize, first: u32, last: u32, cx: &mut Ctx) -> Result<(), String> {
    let (i, (c0, c1), (d0, d1)) = (book.tables[k].sheet, book.tables[k].cols_span(), book.tables[k].data_rows());
    // A table keeps one data row: deleting them all leaves the first one, emptied.
    let (from, keep_one) = if first == d0 && last == d1 { (first + 1, true) } else { (first, false) };
    if from <= last {
        let sh = RowShift::Delete { cols: (c0, c1), first: from, last };
        shift::check(book, i, &sh, Some(k))?;
        let rewritten = shift::apply(book, i, &sh, &mut cx.notices)?;
        cx.arrays.remove(&i);
        cx.changed.follow(i, &sh);
        cx.changed.formulas.extend(rewritten);
    }
    if keep_one {
        for col in c0..=c1 {
            let exists = book.store(i).row(first).is_some_and(|r| r.cell(col).is_some());
            if exists {
                write_cell(book, i, first, col, W::Clear, None, cx)?;
            }
        }
        if book.tables[k].range.last.row != first + book.tables[k].totals {
            book.tables[k].range.last.row = first + book.tables[k].totals;
        }
    }
    book.tables[k].changed = true;
    cx.changed.moved.push((i, CellRange::new(CellRef::new(c0, first), CellRef::new(c1, MAX_ROW))));
    cx.changed.tables.insert(book.tables[k].name.clone());
    Ok(())
}

fn fill_formula(book: &mut Book, k: usize, column: &str, f: &str, cx: &mut Ctx) -> Result<(), String> {
    let t = &book.tables[k];
    let ci = t.col_index(column).ok_or_else(|| format!("table {} has no column {column}", t.name))?;
    let file = formula::to_file_form(f.trim(), &t.name);
    let file = file.strip_prefix('=').unwrap_or(&file).to_string();
    let (i, col, (d0, d1)) = (t.sheet, t.range.first.col + ci as u32, t.data_rows());
    let name = t.name.clone();
    book.tables[k].cols[ci].calc = Some(file.clone());
    book.tables[k].changed = true;
    for r in d0..=d1 {
        let st = book.store(i).row(r).and_then(|x| x.cell(col).map(Cell::style));
        write_formula(book, i, r, col, &file, st, cx)?;
    }
    if let Some(c) = cx.cols.get_mut(&name.to_lowercase()).and_then(|v| v.get_mut(ci)) {
        c.3 = f.trim().to_string();
    }
    cx.changed.tables.insert(name);
    Ok(())
}

/// A displayed text read back as a number (`1,234`, `92.3%`, `00417`).
fn number_of(s: &str) -> Option<f64> {
    let t = s.trim();
    let (t, pct) = match t.strip_suffix('%') {
        Some(x) => (x, true),
        None => (t, false),
    };
    let plain = t.replace(',', "");
    let ok = !plain.is_empty()
        && plain.chars().enumerate().all(|(k, c)| c.is_ascii_digit() || c == '.' || (k == 0 && (c == '-' || c == '+')));
    let v: f64 = if ok { plain.parse().ok()? } else { return None };
    Some(if pct { v / 100.0 } else { v })
}

fn set_type(
    book: &mut Book,
    k: usize,
    column: &str,
    ty: &str,
    format: Option<&str>,
    cx: &mut Ctx,
) -> Result<(), String> {
    let t = &book.tables[k];
    let tname = t.name.clone();
    let ci = t.col_index(column).ok_or_else(|| format!("table {tname} has no column {column}"))?;
    let (i, col, (d0, d1)) = (t.sheet, t.range.first.col + ci as u32, t.data_rows());
    let old = cols_of(cx, t)[ci].clone();
    let fmt = match format {
        Some(f) => f.to_string(),
        None if ty == "text" => "@".into(),
        None => old.2.clone(),
    };
    let is_formula = !old.3.is_empty();
    let rows = book.store(i).rows_in(d0, d1);
    let sheet = book.sheets[i].name.clone();
    for row in rows {
        let Some(cell) = row.cell(col) else { continue };
        let style = book.styles.with_format(cell.style(), &fmt)?;
        if is_formula {
            if style != cell.style() {
                book.store_mut(i)?.row_mut(row.r).cell_mut(col).set("s", &style.to_string());
            }
            continue;
        }
        let v = book.value(cell);
        let at = format!("{sheet}!{}{}", col_letters(col), row.r);
        let shown = book.display(cell);
        let w = match (&v, ty) {
            (CellValue::Empty, _) => W::Clear,
            (CellValue::Text(s), "text") => W::Str(s.clone()),
            (_, "text") => W::Str(shown),
            (CellValue::Number(n), "number") if old.1 != "date" => W::Num(*n),
            (CellValue::Number(n), "date") if old.1 == "date" => W::Num(*n),
            (CellValue::Text(s), "number") => W::Num(
                number_of(s)
                    .ok_or_else(|| format!("{at} holds {s:?}, which is not a number as displayed; set it first"))?,
            ),
            (CellValue::Text(s), "date") => {
                let (y, m, d) = date_of(s)
                    .ok_or_else(|| format!("{at} holds {s:?}, which is not a date YYYY-MM-DD; set it first"))?;
                W::Num(numfmt::serial_of(y as i64, m, d, book.date1904))
            }
            (v, _) => return Err(format!("{at} holds {}, which a {ty} column cannot hold; set it first", show(v))),
        };
        write_cell(book, i, row.r, col, w, Some(style), cx)?;
    }
    let dxf = book.styles.dxf_with_format(&fmt)?;
    book.tables[k].cols[ci].dxf = Some(dxf);
    book.tables[k].changed = true;
    if let Some(c) = cx.cols.get_mut(&tname.to_lowercase()).and_then(|v| v.get_mut(ci)) {
        c.1 = ty.to_string();
        c.2 = fmt;
    }
    cx.changed.tables.insert(tname);
    Ok(())
}

fn show(v: &CellValue) -> String {
    match v {
        CellValue::Empty => "nothing".into(),
        CellValue::Number(n) => numfmt::general(*n),
        CellValue::Text(s) => format!("{s:?}"),
        CellValue::Bool(b) => (if *b { "TRUE" } else { "FALSE" }).into(),
        CellValue::Error(e) => e.clone(),
    }
}

/// Whether cells of sheet `i` in `area` hold anything, or another table or a merge is there.
fn area_problem(book: &Book, i: usize, area: &CellRange, except: Option<usize>) -> Option<String> {
    if let Some(t) = book
        .tables
        .iter()
        .enumerate()
        .find(|(k, t)| t.sheet == i && Some(*k) != except && t.range.intersects(area))
        .map(|x| x.1)
    {
        return Some(format!("table {} ({}) is there; tables on a sheet never overlap", t.name, t.range));
    }
    for e in book.entries.iter().filter(|e| e.kind == Kind::Range && e.path[0] == i && e.meta.tag == "mergeCell") {
        if let Some(m) = e.meta.range.as_deref().and_then(CellRange::parse) {
            if m.intersects(area) {
                return Some(format!("the merged cells {m} are there"));
            }
        }
    }
    for row in book.store(i).rows_in(area.first.row, area.last.row) {
        if let Some(c) = row.cells.iter().find(|c| (area.first.col..=area.last.col).contains(&c.col) && !c.is_blank()) {
            return Some(format!("cell {}{} holds {}", col_letters(c.col), row.r, show(&book.value(c))));
        }
    }
    None
}

fn add_column(book: &mut Book, k: usize, spec: &ColumnSpec, cx: &mut Ctx) -> Result<(), String> {
    let t = &book.tables[k];
    let (i, tname) = (t.sheet, t.name.clone());
    let col = t.range.last.col + 1;
    let (r0, r1, (d0, d1)) = (t.range.first.row, t.range.last.row, t.data_rows());
    let area = CellRange::new(CellRef::new(col, r0), CellRef::new(col, r1));
    book.load_store(i)?;
    if let Some(p) = area_problem(book, i, &area, Some(k)) {
        return Err(format!("the column after table {tname} ({}) is not free: {p}", col_letters(col)));
    }
    let fmt = spec.format.clone().unwrap_or_else(|| if spec.ty == "text" { "@".into() } else { "General".into() });
    let file = spec.formula.as_ref().map(|f| {
        let x = formula::to_file_form(f.trim(), &tname);
        x.strip_prefix('=').unwrap_or(&x).to_string()
    });
    if book.tables[k].header {
        let hs = book.store(i).row(r0).and_then(|r| r.cell(col - 1).map(Cell::style));
        write_cell(book, i, r0, col, W::Str(spec.name.clone()), hs, cx)?;
    }
    for r in d0..=d1 {
        let left = book.store(i).row(r).and_then(|x| x.cell(col - 1).map(Cell::style)).unwrap_or(0);
        let style = book.styles.with_format(left, &fmt)?;
        match &file {
            Some(f) => write_formula(book, i, r, col, f, Some(style), cx)?,
            None if style != 0 => write_cell(book, i, r, col, W::Clear, Some(style), cx)?,
            None => {}
        }
    }
    let dxf = book.styles.dxf_with_format(&fmt)?;
    let t = &mut book.tables[k];
    t.cols.push(TableCol { name: spec.name.clone(), calc: file, dxf: Some(dxf) });
    t.range.last.col = col;
    t.changed = true;
    cx.cols.entry(tname.to_lowercase()).or_default().push((
        spec.name.clone(),
        spec.ty.clone(),
        fmt,
        spec.formula.clone().unwrap_or_default(),
    ));
    cx.changed.tables.insert(tname);
    Ok(())
}

/// Excel's sort order: numbers, text (case-insensitive), FALSE, TRUE, errors; empty cells last either way.
fn sort_key(v: &CellValue) -> (u8, f64, String) {
    match v {
        CellValue::Number(n) => (0, *n, String::new()),
        CellValue::Text(s) => (1, 0.0, s.to_lowercase()),
        CellValue::Bool(b) => (2, f64::from(u8::from(*b)), String::new()),
        CellValue::Error(e) => (3, 0.0, e.clone()),
        CellValue::Empty => (4, 0.0, String::new()),
    }
}

fn sort(book: &mut Book, k: usize, keys: &[SortKey], cx: &mut Ctx) -> Result<(), String> {
    let t = &book.tables[k];
    let (i, (c0, c1), (a, b), tname) = (t.sheet, t.cols_span(), t.data_rows(), t.name.clone());
    if b <= a {
        return Ok(());
    }
    let block = CellRange::new(CellRef::new(c0, a), CellRef::new(c1, b));
    let sheet = book.sheets[i].name.clone();
    for e in book.entries.iter().filter(|e| e.kind == Kind::Range && e.path[0] == i) {
        for r in parse_sqref(e.meta.range.as_deref().unwrap_or("")).unwrap_or_default() {
            let covers = r.first.row <= a && r.last.row >= b;
            if r.intersects(&block) && !covers {
                return Err(format!("{} {r} on sheet {sheet} covers some of the rows being sorted but not all; it would not follow them", shift::describe(&e.meta.tag)));
            }
        }
    }
    book.load_store(i)?;
    let notes =
        book.rels_of(&book.sheets[i].part).iter().any(|r| matches!(r.short_type(), "comments" | "threadedComment"));
    let mut arrays = false;
    book.store(i).for_each_cell(&mut |r, c| {
        if (a..=b).contains(&r)
            && (c0..=c1).contains(&c.col)
            && c.f.as_ref().is_some_and(|f| matches!(f.get("t").as_deref(), Some("array") | Some("dataTable")))
        {
            arrays = true;
        }
    });
    if arrays {
        return Err(format!("the rows of {tname} hold an array formula, which the engine does not move"));
    }
    if notes {
        let refs = note_refs(book, i);
        if refs.iter().any(|c| block.contains(*c)) {
            return Err(format!("notes on rows of {tname} would not follow a sort; moving notes is not supported yet"));
        }
    }
    shift::unshare(book, i)?;
    let cols = book.tables[k].cols.clone();
    let kidx: Vec<(u32, bool)> = keys
        .iter()
        .map(|kk| {
            cols.iter()
                .position(|c| c.name == kk.column)
                .map(|p| (c0 + p as u32, kk.desc))
                .ok_or_else(|| format!("no column {}", kk.column))
        })
        .collect::<Result<_, _>>()?;
    let rows = book.store(i).rows_in(a, b);
    let by_row: HashMap<u32, &crate::store::Row> = rows.iter().map(|r| (r.r, r)).collect();
    let keyrow = |r: u32| -> Vec<(u8, f64, String)> {
        kidx.iter()
            .map(|(c, _)| {
                by_row
                    .get(&r)
                    .and_then(|x| x.cell(*c))
                    .map_or((4, 0.0, String::new()), |cell| sort_key(&book.value(cell)))
            })
            .collect()
    };
    let mut order: Vec<u32> = (a..=b).collect();
    let keys_of: HashMap<u32, Vec<(u8, f64, String)>> = order.iter().map(|&r| (r, keyrow(r))).collect();
    order.sort_by(|x, y| {
        let (kx, ky) = (&keys_of[x], &keys_of[y]);
        for (j, (_, desc)) in kidx.iter().enumerate() {
            let (p, q) = (&kx[j], &ky[j]);
            let ord = if p.0 == 4 || q.0 == 4 {
                p.0.cmp(&q.0)
            } else {
                let o =
                    p.0.cmp(&q.0).then(p.1.partial_cmp(&q.1).unwrap_or(std::cmp::Ordering::Equal)).then(p.2.cmp(&q.2));
                if *desc {
                    o.reverse()
                } else {
                    o
                }
            };
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });
    if order.iter().copied().eq(a..=b) {
        return Ok(());
    }
    // Move the block's cells: new row a+j takes old row order[j]'s cells.
    let st = book.store_mut(i)?;
    let taken = st.take_rows(a, b);
    let mut stay: HashMap<u32, crate::store::Row> = HashMap::new();
    let mut moving: HashMap<u32, Vec<Cell>> = HashMap::new();
    for mut row in taken {
        let (inside, outside): (Vec<Cell>, Vec<Cell>) = row.cells.drain(..).partition(|c| (c0..=c1).contains(&c.col));
        row.cells = outside;
        moving.insert(row.r, inside);
        stay.insert(row.r, row);
    }
    let mut back = vec![];
    for (j, old) in order.iter().enumerate() {
        let nr = a + j as u32;
        let mut row = stay.remove(&nr).unwrap_or_else(|| crate::store::Row::new(nr));
        for mut c in moving.remove(old).unwrap_or_default() {
            c.set_row(nr);
            if let Some(text) = c.formula() {
                let moved = shift::translate(&text, nr as i64 - *old as i64, 0);
                if moved != text {
                    c.f.as_mut().unwrap().children = vec![Node::Text(xml::escape_text(&moved))];
                }
            }
            row.cells.push(c);
        }
        row.cells.sort_by_key(|c| c.col);
        if !shift::is_plain(&row) {
            row.fix_spans();
            back.push(row);
        }
    }
    back.extend(stay.into_values());
    st.put_rows(back);
    // Hyperlinks on single cells of the block go with their rows.
    let pos: HashMap<u32, u32> = order.iter().enumerate().map(|(j, old)| (*old, a + j as u32)).collect();
    for e in book.entries.iter_mut().filter(|e| e.kind == Kind::Range && e.path[0] == i && e.meta.tag == "hyperlink") {
        if let Some(r) = e.meta.range.as_deref().and_then(CellRange::parse) {
            if r.first == r.last && block.contains(r.first) {
                e.meta.range = Some(CellRef::new(r.first.col, pos[&r.first.row]).to_string());
            }
        }
    }
    shift::drop_calc_chain(book);
    cx.changed.areas.push((i, block));
    cx.changed.tables.insert(tname);
    Ok(())
}

fn note_refs(book: &Book, i: usize) -> Vec<CellRef> {
    let part = &book.sheets[i].part;
    let mut out = vec![];
    for r in book.rels_of(part).iter().filter(|r| matches!(r.short_type(), "comments" | "threadedComment")) {
        let target = hanji_package::opc::resolve_target(part, &r.target);
        if let Some(d) = hanji_package::package::get(&book.parts, &target).and_then(|d| xml::parse(d).ok()) {
            d.root.walk(&mut |e| {
                if let Some(c) = (matches!(e.local(), "comment" | "threadedComment"))
                    .then(|| e.get("ref"))
                    .flatten()
                    .and_then(|x| CellRef::parse(&x))
                {
                    out.push(c);
                }
            });
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn add_table(
    book: &mut Book,
    sheet: &str,
    name: &str,
    anchor: &str,
    columns: &[ColumnSpec],
    rows: &[Row],
    cx: &mut Ctx,
) -> Result<(), String> {
    let i = sheet_of(book, sheet)?;
    if book.sheets[i].kind != SheetKind::Work {
        return Err(format!("sheet {sheet} is a chart sheet"));
    }
    book.load_store(i)?;
    let a = CellRef::parse(&anchor.replace('$', "")).ok_or_else(|| format!("{anchor} is not a cell"))?;
    let w = columns.len() as u32;
    let last_row = a.row + (rows.len() as u32).max(1);
    let area = CellRange::new(a, CellRef::new(a.col + w - 1, last_row));
    if let Some(p) = area_problem(book, i, &area, None) {
        return Err(format!("the table's area {area} on sheet {sheet} is not free: {p}"));
    }
    // The new table part.
    let id =
        book.tables.iter().filter_map(|t| t.doc.root.get("id").and_then(|v| v.parse::<u32>().ok())).max().unwrap_or(0)
            + 1;
    let dir = book.wb_part.rsplit_once('/').map_or(String::new(), |x| format!("{}/", x.0));
    let part = book.free_part(&format!("{dir}tables"), "table");
    let src = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<table xmlns=\"{MAIN_NS}\" id=\"{id}\" name=\"{n}\" displayName=\"{n}\" ref=\"{area}\" totalsRowShown=\"0\"><autoFilter ref=\"{area}\"/><tableColumns count=\"0\"/><tableStyleInfo name=\"TableStyleMedium2\" showFirstColumn=\"0\" showLastColumn=\"0\" showRowStripes=\"1\" showColumnStripes=\"0\"/></table>",
        n = xml::escape_attr(name)
    );
    let doc = xml::parse(src.as_bytes()).map_err(|e| e.to_string())?;
    let mut cols = vec![];
    let mut decl: Cols = vec![];
    for c in columns {
        let fmt = c.format.clone().unwrap_or_else(|| if c.ty == "text" { "@".into() } else { "General".into() });
        let calc = c.formula.as_ref().map(|f| {
            let x = formula::to_file_form(f.trim(), name);
            x.strip_prefix('=').unwrap_or(&x).to_string()
        });
        cols.push(TableCol { name: c.name.clone(), calc, dxf: Some(book.styles.dxf_with_format(&fmt)?) });
        decl.push((c.name.clone(), c.ty.clone(), fmt, c.formula.clone().unwrap_or_default()));
    }
    let info = TableInfo {
        part: part.clone(),
        sheet: i,
        doc,
        name: name.to_string(),
        range: area,
        header: true,
        totals: 0,
        cols,
        changed: true,
        new: true,
    };
    book.tables.push(info);
    cx.cols.insert(name.to_lowercase(), decl.clone());
    let sheet_part = book.sheets[i].part.clone();
    let rid = book.add_rel(&sheet_part, "table", &part);
    {
        let st = book.store_mut(i)?;
        let p = st.prefix.clone();
        let root = &mut st.skel.root;
        let rp = match root.attrs.iter().find(|x| x.1 == R_NS).and_then(|x| x.0.strip_prefix("xmlns:")) {
            Some(p) => p.to_string(),
            None => {
                root.attrs.push(("xmlns:r".into(), R_NS.into()));
                "r".into()
            }
        };
        if !root.elements().any(|e| e.local() == "tableParts") {
            xml::insert_ordered(root, Element::new(&format!("{p}tableParts")).with_attr("count", "0"), SHEET_ORDER);
        }
        let tp = root.elements_mut().find(|e| e.local() == "tableParts").unwrap();
        tp.children.push(Node::El(Element::new(&format!("{p}tablePart")).with_attr(&format!("{rp}:id"), &rid)));
        let n = tp.elements().count();
        tp.set("count", &n.to_string());
    }
    // Header and cells.
    for (ci, c) in columns.iter().enumerate() {
        write_cell(book, i, a.row, a.col + ci as u32, W::Str(c.name.clone()), None, cx)?;
    }
    let k = book.tables.len() - 1;
    let calcs: Vec<Option<String>> = book.tables[k].cols.iter().map(|c| c.calc.clone()).collect();
    for r in a.row + 1..=last_row {
        let row = rows.get((r - a.row - 1) as usize);
        for (ci, (cname, ty, fmt, _)) in decl.iter().enumerate() {
            let col = a.col + ci as u32;
            let style = book.styles.with_format(0, fmt)?;
            if let Some(f) = &calcs[ci] {
                write_formula(book, i, r, col, f, Some(style), cx)?;
                continue;
            }
            let v = row.and_then(|row| row.iter().find(|(c, _)| c == cname).map(|x| &x.1)).unwrap_or(&Value::Null);
            let at = format!("{sheet}!{}{r}", col_letters(col));
            let w = to_write(book, v, Some(ty), fmt, &at)?;
            write_cell(book, i, r, col, w, Some(style), cx)?;
        }
    }
    cx.changed.tables.insert(name.to_string());
    cx.changed.areas.push((i, area));
    Ok(())
}

fn add_sheet(book: &mut Book, name: &str) -> Result<(), String> {
    let dir = book.wb_part.rsplit_once('/').map_or(String::new(), |x| format!("{}/", x.0));
    let part = book.free_part(&format!("{dir}worksheets"), "sheet");
    let data = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"{MAIN_NS}\" xmlns:r=\"{R_NS}\"><dimension ref=\"A1\"/><sheetViews><sheetView workbookViewId=\"0\"/></sheetViews><sheetFormatPr defaultRowHeight=\"15\"/><sheetData/><pageMargins left=\"0.7\" right=\"0.7\" top=\"0.75\" bottom=\"0.75\" header=\"0.3\" footer=\"0.3\"/></worksheet>"
    );
    book.add_part(&part, data.into_bytes(), CT_SHEET);
    let wb = book.wb_part.clone();
    let rid = book.add_rel(&wb, "worksheet", &part);
    let root = &mut book.wb.root;
    let p = root.name.strip_suffix("workbook").unwrap_or("").to_string();
    let rp = match root.attrs.iter().find(|x| x.1 == R_NS).and_then(|x| x.0.strip_prefix("xmlns:")) {
        Some(p) => p.to_string(),
        None => {
            root.attrs.push(("xmlns:r".into(), R_NS.into()));
            "r".into()
        }
    };
    let sheets = root.elements_mut().find(|e| e.local() == "sheets").ok_or("the workbook has no sheet list")?;
    let id =
        sheets.elements().filter_map(|s| s.get("sheetId").and_then(|v| v.parse::<u32>().ok())).max().unwrap_or(0) + 1;
    sheets.children.push(Node::El(
        Element::new(&format!("{p}sheet"))
            .with_attr("name", name)
            .with_attr("sheetId", &id.to_string())
            .with_attr(&format!("{rp}:id"), &rid),
    ));
    book.wb_changed = true;
    book.push_sheet(SheetInfo { name: name.to_string(), part, rid, kind: SheetKind::Work, state: "visible".into() });
    Ok(())
}

/// The sheet's `<dimension>` after its cells changed.
fn fix_dimension(book: &mut Book, i: usize) -> Result<(), String> {
    let st = book.store_mut(i)?;
    let used = st
        .used()
        .map(|(c0, r0, c1, r1)| CellRange::new(CellRef::new(c0, r0), CellRef::new(c1, r1)).to_string())
        .unwrap_or_else(|| "A1".into());
    let p = st.prefix.clone();
    match st.child_mut("dimension") {
        Some(d) => {
            if d.get("ref").as_deref() != Some(used.as_str()) {
                d.set("ref", &used);
            }
        }
        None => xml::insert_ordered(
            &mut st.skel.root,
            Element::new(&format!("{p}dimension")).with_attr("ref", &used),
            SHEET_ORDER,
        ),
    }
    Ok(())
}
