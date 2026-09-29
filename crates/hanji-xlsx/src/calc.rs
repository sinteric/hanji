//! Formula results (§5.4: computed before export, so a viewer that does not
//! recalculate does not show 0). After range operations, the formulas whose
//! inputs changed — found through their references: A1, structured and
//! defined names — are computed by IronCalc over the engine's own cells, and
//! their cached `<v>` values written; nothing else in the file changes and
//! IronCalc never reads or writes the package. A formula IronCalc cannot
//! compute (a function it lacks, an array formula, a volatile function) keeps
//! its cached value, and the workbook asks to be recalculated when opened.

use std::collections::{BTreeMap, HashMap, HashSet};

use hanji_core::cells::{CellRange, CellRef, MAX_COL, MAX_ROW};
use hanji_format::formula::{self, Tok};
use hanji_package::xml::{self, Element};
use ironcalc_base::expressions::types::CellReferenceRC;

use crate::book::{Book, SheetKind};
use crate::shift::translate;
use crate::store::has_formula;
use crate::value::CellValue;

/// Where operations changed the book: cells whose values changed, areas
/// whose cells moved, and formula cells written or rewritten.
#[derive(Clone, Debug, Default)]
pub struct Changed {
    pub cells: HashSet<(usize, u32, u32)>,
    /// Areas whose cells changed in place (a sort, a new table).
    pub areas: Vec<(usize, CellRange)>,
    /// Areas whose rows moved (inserted or deleted rows and those below them):
    /// their cells keep their values, but a range over them may change extent.
    pub moved: Vec<(usize, CellRange)>,
    pub formulas: HashSet<(usize, u32, u32)>,
    /// Tables whose rows or columns changed: a reference to them reads new cells.
    pub tables: HashSet<String>,
}

impl Changed {
    /// Follow rows of sheet `i` that `sh` moves: what changed there before is where it now is.
    pub fn follow(&mut self, i: usize, sh: &hanji_core::cells::RowShift) {
        use hanji_core::cells::Moved;
        let at =
            |&(s, r, c): &(usize, u32, u32)| if s == i { sh.row(c, r).map(|r| (s, r, c)) } else { Some((s, r, c)) };
        self.cells = self.cells.iter().filter_map(at).collect();
        self.formulas = self.formulas.iter().filter_map(at).collect();
        for list in [&mut self.areas, &mut self.moved] {
            *list = list
                .iter()
                .filter_map(|&(s, a)| match (s == i).then(|| sh.range(&a)) {
                    Some(Moved::To(to)) => Some((s, to)),
                    Some(Moved::Gone) => None,
                    // Torn areas (a moved area reaching past the columns) keep their place.
                    _ => Some((s, a)),
                })
                .collect();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
            && self.areas.is_empty()
            && self.moved.is_empty()
            && self.formulas.is_empty()
            && self.tables.is_empty()
    }
}

/// What recalculation did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recalc {
    /// Formula cells whose inputs changed.
    pub dirty: usize,
    /// Of those, cached values written.
    pub written: usize,
    /// Of those, left to the application (it recalculates when opened).
    pub left: Vec<String>,
}

/// Functions whose value changes every time: their cells keep the cached value.
const VOLATILE: &[&str] = &["NOW", "TODAY", "RAND", "RANDBETWEEN", "RANDARRAY"];

#[derive(Clone, Debug)]
enum Prec {
    Area(usize, CellRange),
    /// Anything (INDIRECT, OFFSET, a name that cannot be resolved).
    Unknown,
}

struct Formula {
    sheet: usize,
    row: u32,
    col: u32,
    /// Text as the file stores it (shared formulas expanded).
    text: String,
    /// IronCalc cannot compute it: an array or data-table formula, or a volatile one.
    constant: bool,
    precs: Vec<Prec>,
    /// Its value as the file has it, and whether it has none.
    cached: CellValue,
    uncached: bool,
}

/// The book's formula cells, with their texts and references.
fn formulas(book: &mut Book) -> Result<Vec<Formula>, String> {
    let mut out = vec![];
    for i in 0..book.sheets.len() {
        if book.sheets[i].kind != SheetKind::Work {
            continue;
        }
        book.load_store(i)?;
        let mut masters: HashMap<String, (u32, u32, String)> = HashMap::new();
        // (row, column, `t`, the `si` of a shared formula's follower, text, cached value, whether it has none)
        type Found = (u32, u32, String, Option<String>, String, CellValue, bool);
        let mut cells: Vec<Found> = vec![];
        let st = book.store(i);
        st.for_each_cell_if(&has_formula, &mut |r, c| {
            let Some(f) = &c.f else { return };
            let t = f.get("t").unwrap_or_default();
            let text = f.text_of(&[f.name.as_str()]);
            let si = f.get("si").filter(|_| t == "shared");
            if si.is_some() && f.get("ref").is_some() {
                masters.insert(si.clone().unwrap_or_default(), (r, c.col, text.clone()));
            }
            let follower = si.filter(|_| f.get("ref").is_none());
            cells.push((r, c.col, t, follower, text, book.value(c), c.lacks_cached_value()));
        });
        for (r, col, t, follower, mut text, cached, uncached) in cells {
            if let Some(si) = follower {
                match masters.get(&si) {
                    Some((mr, mc, mt)) => text = translate(mt, r as i64 - *mr as i64, col as i64 - *mc as i64),
                    None => continue,
                }
            }
            let fns = formula::functions(&formula::tokenize(&text));
            let constant = t == "array" || t == "dataTable" || fns.iter().any(|n| VOLATILE.contains(&n.as_str()));
            out.push(Formula { sheet: i, row: r, col, precs: vec![], text, constant, cached, uncached });
        }
    }
    for f in out.iter_mut() {
        f.precs = precedents(book, f);
    }
    Ok(out)
}

fn precedents(book: &Book, f: &Formula) -> Vec<Prec> {
    let mut out = vec![];
    refs_of(book, &f.text, f.sheet, f.row, f.col, &mut out, 0);
    out
}

fn refs_of(book: &Book, text: &str, sheet: usize, row: u32, col: u32, out: &mut Vec<Prec>, depth: u32) {
    for t in formula::tokenize(text) {
        match t.kind {
            Tok::Ref(r) => {
                let s = match &r.sheet {
                    Some(p) if p.external => continue,
                    Some(p) if p.to.is_some() => {
                        out.push(Prec::Unknown);
                        continue;
                    }
                    Some(p) => book.sheet_index(&p.name),
                    None => Some(sheet),
                };
                let (Some(s), Some(a)) = (s, r.first) else { continue };
                let b = r.last.unwrap_or(a);
                let first = CellRef::new(a.col.unwrap_or(0), a.row.unwrap_or(1));
                let last = CellRef::new(b.col.unwrap_or(MAX_COL - 1), b.row.unwrap_or(MAX_ROW));
                out.push(Prec::Area(s, CellRange::new(first, last)));
            }
            Tok::Structured(sr) => {
                let t = match &sr.table {
                    Some(n) => book.tables.iter().find(|t| t.name.eq_ignore_ascii_case(n)),
                    None => book.tables.iter().find(|t| t.sheet == sheet && t.range.contains(CellRef::new(col, row))),
                };
                let Some(t) = t else {
                    out.push(Prec::Unknown);
                    continue;
                };
                let (c0, c1) = match &sr.columns {
                    Some((a, b)) => match (t.col_index(a), t.col_index(b)) {
                        (Some(x), Some(y)) => {
                            (t.range.first.col + x.min(y) as u32, t.range.first.col + x.max(y) as u32)
                        }
                        _ => {
                            out.push(Prec::Unknown);
                            continue;
                        }
                    },
                    None => (t.range.first.col, t.range.last.col),
                };
                let (d0, d1) = t.data_rows();
                let (r0, r1) = if sr.this_row() {
                    (row, row)
                } else if sr.items.iter().any(|i| i == "#All") {
                    (t.range.first.row, t.range.last.row)
                } else if sr.items.iter().any(|i| i == "#Headers") && sr.items.len() == 1 {
                    (t.range.first.row, t.range.first.row)
                } else if sr.items.iter().any(|i| i == "#Totals") && sr.items.len() == 1 {
                    (t.range.last.row, t.range.last.row)
                } else {
                    (d0, d1.max(d0))
                };
                out.push(Prec::Area(
                    t.sheet,
                    CellRange::new(CellRef::new(c0, r0.min(r1)), CellRef::new(c1, r1.max(r0))),
                ));
            }
            Tok::Func(n) if matches!(formula::bare_function(&n).as_str(), "INDIRECT" | "OFFSET") => {
                out.push(Prec::Unknown)
            }
            Tok::Name(n) => {
                let def = book.wb.root.elements().find(|e| e.local() == "definedNames").and_then(|d| {
                    d.elements()
                        .find(|e| e.get("name").is_some_and(|x| x.eq_ignore_ascii_case(&n)))
                        .map(|e| e.text_of(&[e.name.as_str()]))
                });
                match def {
                    Some(d) if depth < 8 => refs_of(book, &d, sheet, row, col, out, depth + 1),
                    Some(_) => out.push(Prec::Unknown),
                    None => {}
                }
            }
            _ => {}
        }
    }
}

/// The formula cells whose inputs changed.
fn dirty(fs: &[Formula], ch: &Changed, book: &Book) -> Vec<usize> {
    let mut single: HashMap<(usize, u32, u32), Vec<usize>> = HashMap::new();
    let mut ranges: HashMap<(usize, CellRange), Vec<usize>> = HashMap::new();
    let mut unknown = vec![];
    for (k, f) in fs.iter().enumerate() {
        for p in &f.precs {
            match p {
                Prec::Unknown => unknown.push(k),
                Prec::Area(s, r) if r.first == r.last => {
                    single.entry((*s, r.first.row, r.first.col)).or_default().push(k)
                }
                Prec::Area(s, r) => ranges.entry((*s, *r)).or_default().push(k),
            }
        }
    }
    let at: HashMap<(usize, u32, u32), usize> =
        fs.iter().enumerate().map(|(k, f)| ((f.sheet, f.row, f.col), k)).collect();
    let mut is_dirty = vec![false; fs.len()];
    let mut queue: Vec<usize> = vec![];
    let mark = |k: usize, q: &mut Vec<usize>, d: &mut Vec<bool>| {
        if !d[k] {
            d[k] = true;
            q.push(k);
        }
    };
    let anything = !ch.is_empty();
    if anything {
        for &k in &unknown {
            mark(k, &mut queue, &mut is_dirty);
        }
    }
    for c in &ch.formulas {
        if let Some(&k) = at.get(c) {
            mark(k, &mut queue, &mut is_dirty);
        }
    }
    let table_areas: Vec<(usize, CellRange)> =
        book.tables.iter().filter(|t| ch.tables.contains(&t.name)).map(|t| (t.sheet, t.range)).collect();
    let mut seed_cells: Vec<(usize, u32, u32)> = ch.cells.iter().copied().collect();
    seed_cells.sort();
    for &(s, r, c) in &seed_cells {
        for k in single.get(&(s, r, c)).into_iter().flatten() {
            mark(*k, &mut queue, &mut is_dirty);
        }
    }
    for ((s, r), ks) in &ranges {
        let hit = seed_cells.iter().any(|&(cs, cr, cc)| cs == *s && r.contains(CellRef::new(cc, cr)))
            || ch.areas.iter().chain(&ch.moved).chain(&table_areas).any(|(a, ar)| a == s && ar.intersects(r));
        if hit {
            for k in ks {
                mark(*k, &mut queue, &mut is_dirty);
            }
        }
    }
    for ((s, r, c), ks) in &single {
        if ch.areas.iter().any(|(a, ar)| a == s && ar.contains(CellRef::new(*c, *r))) {
            for k in ks {
                mark(*k, &mut queue, &mut is_dirty);
            }
        }
    }
    // A single cell that moved kept its value, and a formula reading it was
    // rewritten to follow it (and is in `ch.formulas` then); one that went is
    // `#REF!` in a rewritten formula. So moved areas matter only to ranges,
    // whose extent may change.
    // Formulas reading dirty formulas.
    let range_list: Vec<(&(usize, CellRange), &Vec<usize>)> = ranges.iter().collect();
    while let Some(k) = queue.pop() {
        let f = &fs[k];
        for j in single.get(&(f.sheet, f.row, f.col)).cloned().into_iter().flatten() {
            mark(j, &mut queue, &mut is_dirty);
        }
        for ((s, r), ks) in &range_list {
            if *s == f.sheet && r.contains(CellRef::new(f.col, f.row)) {
                for &j in ks.iter() {
                    mark(j, &mut queue, &mut is_dirty);
                }
            }
        }
    }
    (0..fs.len()).filter(|&k| is_dirty[k]).collect()
}

fn to_ironcalc(text: &str) -> String {
    let mut s = text.replace("_xlfn._xlws.", "").replace("_xlfn.", "").replace("_xlws.", "");
    if !s.starts_with('=') {
        s.insert(0, '=');
    }
    s
}

/// Every formula cell of the book, as changed (to compute them all).
pub fn uncached_formulas(book: &mut Book) -> Result<Changed, String> {
    let mut ch = Changed::default();
    for i in 0..book.sheets.len() {
        if book.sheets[i].kind != SheetKind::Work {
            continue;
        }
        book.load_store(i)?;
        book.store(i).for_each_cell_if(&has_formula, &mut |r, c| {
            if c.lacks_cached_value() {
                ch.formulas.insert((i, r, c.col));
            }
        });
    }
    Ok(ch)
}

/// Functions whose result depends on the date system (IronCalc knows 1900 only).
const EPOCH_FUNCTIONS: &[&str] = &[
    "DATE",
    "DATEVALUE",
    "DAY",
    "MONTH",
    "YEAR",
    "WEEKDAY",
    "WEEKNUM",
    "ISOWEEKNUM",
    "EDATE",
    "EOMONTH",
    "NETWORKDAYS",
    "NETWORKDAYS.INTL",
    "WORKDAY",
    "WORKDAY.INTL",
    "DAYS",
    "DAYS360",
    "DATEDIF",
    "YEARFRAC",
    "TEXT",
    "VALUE",
    "NUMBERVALUE",
    "NOW",
    "TODAY",
    "DATESTRING",
];

/// In a 1904 workbook, the dirty formulas whose value depends on the date
/// system: they call a date function or hold a text constant (which may read
/// as a date), or read such a formula. Arithmetic on serial numbers is the
/// same in both systems, so the others are computed.
fn reads_the_epoch(fs: &[Formula], dirty: &[usize]) -> HashSet<usize> {
    let mut out: HashSet<usize> = dirty
        .iter()
        .copied()
        .filter(|&k| {
            let toks = formula::tokenize(&fs[k].text);
            toks.iter().any(|t| matches!(t.kind, Tok::Str))
                || formula::functions(&toks).iter().any(|n| EPOCH_FUNCTIONS.contains(&n.as_str()))
        })
        .collect();
    loop {
        let reads = |k: usize, out: &HashSet<usize>| {
            fs[k].precs.iter().any(|p| match p {
                Prec::Unknown => !out.is_empty(),
                Prec::Area(s, r) => {
                    out.iter().any(|&j| fs[j].sheet == *s && r.contains(CellRef::new(fs[j].col, fs[j].row)))
                }
            })
        };
        let more: Vec<usize> = dirty.iter().copied().filter(|k| !out.contains(k) && reads(*k, &out)).collect();
        if more.is_empty() {
            return out;
        }
        out.extend(more);
    }
}

/// The calculator's value of each formula cell: (sheet, row, column) → value.
type Values = HashMap<(usize, u32, u32), Option<CellValue>>;

/// Compute the formulas whose inputs changed and write their cached values.
pub fn recompute(book: &mut Book, ch: &Changed) -> Result<Recalc, String> {
    let mut rc = Recalc::default();
    if ch.is_empty() {
        return Ok(rc);
    }
    let fs = formulas(book)?;
    if fs.is_empty() {
        return Ok(rc);
    }
    let d = dirty(&fs, ch, book);
    rc.dirty = d.len();
    if d.is_empty() {
        return Ok(rc);
    }
    let epoch = if book.date1904 { reads_the_epoch(&fs, &d) } else { HashSet::new() };
    let values = evaluate(book, &fs, &d)?;
    // Cached values to write: sheet → row → (column, value).
    let mut writes: BTreeMap<usize, BTreeMap<u32, Vec<(u32, CellValue)>>> = BTreeMap::new();
    for &k in &d {
        let f = &fs[k];
        let at = || format!("{}!{}{}", book.sheets[f.sheet].name, hanji_core::cells::col_letters(f.col), f.row);
        let why = if f.constant {
            Some("volatile or array formula")
        } else if epoch.contains(&k) {
            Some("reads dates in the 1904 system")
        } else {
            None
        };
        let v = if why.is_some() { None } else { values.get(&(f.sheet, f.row, f.col)).cloned().flatten() };
        match v {
            None => rc.left.push(format!("{} ({})", at(), why.unwrap_or("not computed"))),
            Some(CellValue::Error(e))
                if e == "#N/IMPL" || (e == "#NAME?" && f.cached != CellValue::Error("#NAME?".into())) =>
            {
                rc.left.push(format!("{} ({e} in the calculator)", at()))
            }
            Some(v) if v != f.cached || f.uncached => {
                writes.entry(f.sheet).or_default().entry(f.row).or_default().push((f.col, v));
                rc.written += 1;
            }
            Some(_) => {}
        }
    }
    for (s, by_row) in writes {
        let st = book.store_mut(s)?;
        let rows: Vec<u32> = by_row.keys().copied().collect();
        st.edit_rows(&rows, &mut |row| {
            for (c, v) in by_row.get(&row.r).into_iter().flatten() {
                write_cached(row.cell_mut(*c), v);
            }
        });
    }
    if !rc.left.is_empty() {
        book.shell.recalc_on_open = true;
    }
    Ok(rc)
}

/// A formula cell's cached value (`t` and `<v>`), its formula untouched.
pub fn write_cached(cell: &mut crate::store::Cell, v: &CellValue) {
    match v {
        CellValue::Number(n) => {
            cell.remove("t");
            cell.v = Some(num_text(*n));
        }
        CellValue::Text(s) => {
            cell.set("t", "str");
            cell.v = Some(crate::sst::encode_xstring(s));
        }
        CellValue::Bool(b) => {
            cell.set("t", "b");
            cell.v = Some(if *b { "1" } else { "0" }.into());
        }
        CellValue::Error(e) => {
            cell.set("t", "e");
            cell.v = Some(xml::escape_text(e));
        }
        CellValue::Empty => {
            cell.remove("t");
            cell.v = None;
        }
    }
    cell.is = None;
}

/// A number as `<v>` writes it (the shortest text that reads back to it).
pub fn num_text(n: f64) -> String {
    if n == n.trunc() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// IronCalc's value of every formula cell of the book.
fn evaluate(book: &Book, fs: &[Formula], dirty: &[usize]) -> Result<Values, String> {
    use ironcalc_base::expressions::parser::new_parser_english;
    use ironcalc_base::types::Cell as C;
    use ironcalc_base::types::{
        DefinedName, Metadata, SheetState, Table, TableColumn, TableStyleInfo, Workbook, WorkbookSettings,
        WorkbookView, Worksheet, WorksheetView,
    };
    use ironcalc_base::Model;
    let mut worksheets = vec![];
    for (i, s) in book.sheets.iter().enumerate() {
        let mut views = HashMap::new();
        views.insert(0, WorksheetView { row: 1, column: 1, range: [1, 1, 1, 1], top_row: 1, left_column: 1 });
        worksheets.push(Worksheet {
            dimension: "A1".into(),
            cols: vec![],
            rows: vec![],
            name: s.name.clone(),
            sheet_data: Default::default(),
            shared_formulas: vec![],
            sheet_id: i as u32 + 1,
            state: SheetState::Visible,
            color: None,
            merge_cells: vec![],
            comments: vec![],
            frozen_rows: 0,
            frozen_columns: 0,
            views,
            show_grid_lines: true,
        });
    }
    let mut tables = HashMap::new();
    for t in &book.tables {
        tables.insert(
            t.name.clone(),
            Table {
                name: t.name.clone(),
                display_name: t.name.clone(),
                sheet_name: book.sheets[t.sheet].name.clone(),
                reference: t.range.to_string(),
                totals_row_count: t.totals,
                header_row_count: u32::from(t.header),
                header_row_dxf_id: None,
                data_dxf_id: None,
                totals_row_dxf_id: None,
                columns: t
                    .cols
                    .iter()
                    .enumerate()
                    .map(|(k, c)| TableColumn { id: k as u32 + 1, name: c.name.clone(), ..Default::default() })
                    .collect(),
                style_info: TableStyleInfo::default(),
                has_filters: false,
            },
        );
    }
    let mut defined = vec![];
    if let Some(dn) = book.wb.root.elements().find(|e| e.local() == "definedNames") {
        for d in dn.elements() {
            let name = d.get("name").unwrap_or_default();
            if name.starts_with("_xlnm.") {
                continue;
            }
            let sheet_id = d.get("localSheetId").and_then(|v| v.parse::<u32>().ok()).map(|v| v + 1);
            defined.push(DefinedName { name, formula: d.text_of(&[d.name.as_str()]).replace("_xlfn.", ""), sheet_id });
        }
    }
    let mut views = HashMap::new();
    views.insert(0, WorkbookView { sheet: 0, window_width: 800, window_height: 600 });
    let wb = Workbook {
        shared_strings: vec![],
        defined_names: defined,
        worksheets,
        styles: Default::default(),
        name: "hanji".into(),
        settings: WorkbookSettings { tz: "UTC".into(), locale: "en".into() },
        metadata: Metadata {
            application: "hanji".into(),
            app_version: String::new(),
            creator: String::new(),
            last_modified_by: String::new(),
            created: String::new(),
            last_modified: String::new(),
        },
        tables,
        views,
    };
    // The cells go straight into the workbook, each formula parsed once and
    // stored once per distinct R1C1 form (as IronCalc's own loader does; its
    // cell-by-cell setters search the formula list, quadratic in the cells).
    let mut wb = wb;
    let sheet_names: Vec<String> = wb.worksheets.iter().map(|w| w.name.clone()).collect();
    let mut parser = new_parser_english(sheet_names, wb.get_defined_names_with_scope(), wb.tables.clone());
    // The calculator gets the dirty formulas and the rows they read; every
    // other formula is its cached value. A dirty formula that may read
    // anything (INDIRECT, OFFSET), or an input formula without a cached value,
    // gets the whole book, every formula computed.
    let live: HashSet<(usize, u32, u32)> = dirty.iter().map(|&k| (fs[k].sheet, fs[k].row, fs[k].col)).collect();
    let mut spans: Spans = HashMap::new();
    let mut all = false;
    for &k in dirty {
        let f = &fs[k];
        spans.entry(f.sheet).or_default().push((f.row, f.row));
        for p in &f.precs {
            match p {
                Prec::Unknown => all = true,
                Prec::Area(s, r) => spans.entry(*s).or_default().push((r.first.row, r.last.row)),
            }
        }
    }
    for v in spans.values_mut() {
        v.sort_unstable();
        let mut merged: Vec<(u32, u32)> = vec![];
        for &(a, b) in v.iter() {
            match merged.last_mut() {
                Some(last) if a <= last.1.saturating_add(1) => last.1 = last.1.max(b),
                _ => merged.push((a, b)),
            }
        }
        *v = merged;
    }
    let partial = if all { None } else { load_cells(book, fs, Some((&live, &spans)), &mut parser) };
    let Cells { sheets, strings, errors } =
        partial.unwrap_or_else(|| load_cells(book, fs, None, &mut parser).expect("the whole book loads"));
    for (i, (data, shared)) in sheets.into_iter().enumerate() {
        wb.worksheets[i].sheet_data = data;
        wb.worksheets[i].shared_formulas = shared;
    }
    let mut ss = vec![String::new(); strings.len()];
    for (t, k) in strings {
        ss[k as usize] = t;
    }
    wb.shared_strings = ss;
    let mut m = match Model::from_workbook(wb, "en") {
        Ok(m) => m,
        Err(_) => return Ok(HashMap::new()),
    };
    for (s, row, col, e) in errors {
        let _ = m.set_user_input(s, row, col, e);
    }
    m.evaluate();
    let mut out = HashMap::new();
    for f in dirty.iter().map(|&k| &fs[k]) {
        let v = m
            .workbook
            .worksheet(f.sheet as u32)
            .ok()
            .and_then(|w| w.cell(f.row as i32, f.col as i32 + 1).cloned())
            .map(|c| match c {
                C::CellFormulaNumber { v, .. } | C::NumberCell { v, .. } => CellValue::Number(v),
                C::CellFormulaBoolean { v, .. } | C::BooleanCell { v, .. } => CellValue::Bool(v),
                C::CellFormulaString { v, .. } => CellValue::Text(v),
                C::CellFormulaError { ei, .. } | C::ErrorCell { ei, .. } => CellValue::Error(ei.to_string()),
                C::SharedString { si, .. } => {
                    m.workbook.shared_strings.get(si as usize).cloned().map_or(CellValue::Empty, CellValue::Text)
                }
                C::EmptyCell { .. } => CellValue::Empty,
                C::CellFormula { .. } => CellValue::Error("#N/IMPL".into()),
            });
        out.insert((f.sheet, f.row, f.col), v);
    }
    Ok(out)
}

/// A cell: sheet, row, column.
type Key = (usize, u32, u32);

/// Row spans (first, last) per sheet.
type Spans = HashMap<usize, Vec<(u32, u32)>>;

/// The cells IronCalc gets: each sheet's cells and its distinct formulas
/// (R1C1), the shared strings, and error cells (set after loading).
struct Cells {
    sheets: Vec<(ironcalc_base::types::SheetData, Vec<String>)>,
    strings: HashMap<String, i32>,
    errors: Vec<(u32, i32, i32, String)>,
}

/// The dirty formulas (`live`) and the rows they read (`spans`), every other
/// formula as its cached value; `None` when one of those has no cached value.
/// With `only` `None`, every cell and every formula.
fn load_cells(
    book: &Book,
    fs: &[Formula],
    only: Option<(&HashSet<Key>, &Spans)>,
    parser: &mut ironcalc_base::expressions::parser::Parser<'_>,
) -> Option<Cells> {
    use ironcalc_base::types::Cell as C;
    let formula_at: HashMap<(usize, u32, u32), &Formula> = fs.iter().map(|f| ((f.sheet, f.row, f.col), f)).collect();
    let mut out = Cells { sheets: vec![], strings: HashMap::new(), errors: vec![] };
    let whole = vec![(1, MAX_ROW)];
    for i in 0..book.sheets.len() {
        let mut index: HashMap<String, i32> = HashMap::new();
        let (mut data, mut shared): (ironcalc_base::types::SheetData, Vec<String>) = (HashMap::new(), vec![]);
        let spans = match only {
            _ if book.sheets[i].kind != SheetKind::Work => None,
            None => Some(&whole),
            Some((_, spans)) => spans.get(&i),
        };
        let mut missing = false;
        for &(a, b) in spans.into_iter().flatten() {
            book.store(i).for_each_row_in(a, b, &mut |row| {
                for c in &row.cells {
                    if missing {
                        return;
                    }
                    let (rr, col) = (row.r as i32, c.col as i32 + 1);
                    let key = (i, row.r, c.col);
                    let f = formula_at
                        .get(&key)
                        .filter(|f| !f.constant && only.is_none_or(|(live, _)| live.contains(&key)));
                    let cell = match f {
                        Some(f) => {
                            let at = CellReferenceRC { sheet: book.sheets[i].name.clone(), row: rr, column: col };
                            C::CellFormula { f: formula_index(parser, &f.text, &at, &mut index, &mut shared), s: 0 }
                        }
                        None if only.is_some() && c.lacks_cached_value() => {
                            missing = true;
                            continue;
                        }
                        None => match book.value(c) {
                            CellValue::Empty => continue,
                            CellValue::Number(v) => C::NumberCell { v, s: 0 },
                            CellValue::Bool(v) => C::BooleanCell { v, s: 0 },
                            CellValue::Text(t) => {
                                let n = out.strings.len() as i32;
                                C::SharedString { si: *out.strings.entry(t).or_insert(n), s: 0 }
                            }
                            CellValue::Error(e) => {
                                out.errors.push((i as u32, rr, col, e));
                                continue;
                            }
                        },
                    };
                    data.entry(rr).or_default().insert(col, cell);
                }
            });
            if missing {
                return None;
            }
        }
        out.sheets.push((data, shared));
    }
    Some(out)
}

/// The index of a formula's R1C1 form among `shared`, added when new. A
/// formula that does not parse is tried with a closing parenthesis, as IronCalc does.
fn formula_index(
    parser: &mut ironcalc_base::expressions::parser::Parser<'_>,
    text: &str,
    at: &CellReferenceRC,
    index: &mut HashMap<String, i32>,
    shared: &mut Vec<String>,
) -> i32 {
    use ironcalc_base::expressions::parser::stringify::to_rc_format;
    use ironcalc_base::expressions::parser::Node;
    let text = to_ironcalc(text);
    let mut node = parser.parse(&text[1..], at);
    if matches!(node, Node::ParseErrorKind { .. }) {
        let again = parser.parse(&format!("{})", &text[1..]), at);
        if !matches!(again, Node::ParseErrorKind { .. }) {
            node = again;
        }
    }
    let rc = to_rc_format(&node);
    let n = shared.len() as i32;
    *index.entry(rc).or_insert_with_key(|rc| {
        shared.push(rc.clone());
        n
    })
}

/// The workbook asks for a full recalculation when opened, if the
/// calculator left formulas to the application.
pub fn mark_recalc(book: &mut Book) {
    if !book.shell.recalc_on_open {
        return;
    }
    let p = book.wb.root.name.strip_suffix("workbook").unwrap_or("").to_string();
    let order = [
        "fileVersion",
        "fileSharing",
        "workbookPr",
        "workbookProtection",
        "bookViews",
        "sheets",
        "functionGroups",
        "externalReferences",
        "definedNames",
        "calcPr",
        "oleSize",
        "customWorkbookViews",
        "pivotCaches",
        "smartTagPr",
        "smartTagTypes",
        "webPublishing",
        "fileRecoveryPr",
        "webPublishObjects",
        "extLst",
    ];
    if !book.wb.root.elements().any(|e| e.local() == "calcPr") {
        xml::insert_ordered(&mut book.wb.root, Element::new(&format!("{p}calcPr")), &order);
    }
    let c = book.wb.root.elements_mut().find(|e| e.local() == "calcPr").unwrap();
    if c.get("fullCalcOnLoad").as_deref() != Some("1") {
        c.set("fullCalcOnLoad", "1");
        book.wb_changed = true;
    }
}
