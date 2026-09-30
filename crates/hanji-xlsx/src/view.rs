//! Row windows (§5.4, §6 round 4 read view A; §2 rule 11): a table's rows or
//! a sheet range as a pipe table whose first column is the sheet row number,
//! blank rows included, every value as displayed. Only the rows a window
//! shows are parsed.

use hanji_core::cells::{col_letters, CellRange};
use hanji_format::sheet::{sheet_open, table_lines, window_text, WindowOf};

use crate::book::{Book, SheetKind};
use crate::model;

/// Most rows one window shows; a larger table is read one window at a time.
pub const MAX_WINDOW: u32 = 500;

/// Displayed values of columns `c0..=c1` for rows `a..=b` of sheet `i`.
fn rows_of(book: &Book, i: usize, c0: u32, c1: u32, a: u32, b: u32) -> Vec<(u32, Vec<String>)> {
    let st = book.store(i);
    let parsed = st.rows_in(a, b);
    let mut it = parsed.iter().peekable();
    let mut out = Vec::with_capacity((b - a + 1) as usize);
    for r in a..=b {
        let row = match it.peek() {
            Some(x) if x.r == r => it.next(),
            _ => None,
        };
        let cells =
            (c0..=c1).map(|c| row.and_then(|x| x.cell(c)).map(|cell| book.display(cell)).unwrap_or_default()).collect();
        out.push((r, cells));
    }
    out
}

pub fn window(book: &mut Book, of: &WindowOf) -> Result<String, String> {
    match of {
        WindowOf::Table { name, rows } => {
            let k = book.table_index(name).ok_or_else(|| {
                let names: Vec<String> = book.tables.iter().map(|t| format!("\"{}\"", t.name)).collect();
                format!("there is no table \"{name}\". Tables: {}.", names.join(", "))
            })?;
            let (i, (first, last)) = (book.tables[k].sheet, book.tables[k].data_rows());
            book.load_store(i)?;
            let t = &book.tables[k];
            let (a, b) = match rows {
                Some((a, b)) => {
                    if *a < first || *b > last || a > b {
                        return Err(format!("rows {a}:{b} are not data rows of {} (rows {first}:{last}).", t.name));
                    }
                    if b - a + 1 > MAX_WINDOW {
                        return Err(format!(
                            "a window shows at most {MAX_WINDOW} rows; ask for {a}:{}.",
                            a + MAX_WINDOW - 1
                        ));
                    }
                    (*a, *b)
                }
                None => (first, last.min(first + MAX_WINDOW - 1)),
            };
            let header: Vec<String> = t.cols.iter().map(|c| c.name.clone()).collect();
            let shown = if rows.is_none() && b == last { None } else { Some((a, b)) };
            let rows = if a <= b { rows_of(book, i, t.range.first.col, t.range.last.col, a, b) } else { vec![] };
            Ok(window_text(&WindowOf::Table { name: t.name.clone(), rows: shown }, &header, &rows))
        }
        WindowOf::Range { sheet, range } => {
            let i = book.sheet_index(sheet).ok_or_else(|| {
                let names: Vec<String> = book.sheets.iter().map(|s| format!("\"{}\"", s.name)).collect();
                format!("there is no sheet \"{sheet}\". Sheets: {}.", names.join(", "))
            })?;
            if book.sheets[i].kind != SheetKind::Work {
                return Err(format!("sheet \"{sheet}\" is a chart sheet; it has no cells."));
            }
            let r = CellRange::parse(range).ok_or_else(|| format!("{range:?} is not a range such as A1:F50."))?;
            if r.rows() > MAX_WINDOW {
                return Err(format!(
                    "a window shows at most {MAX_WINDOW} rows; ask for {}.",
                    CellRange::new(r.first, hanji_core::cells::CellRef::new(r.last.col, r.first.row + MAX_WINDOW - 1))
                ));
            }
            if r.cols() > 64 {
                return Err("a window shows at most 64 columns.".into());
            }
            book.load_store(i)?;
            let header: Vec<String> = (r.first.col..=r.last.col).map(col_letters).collect();
            let rows = rows_of(book, i, r.first.col, r.last.col, r.first.row, r.last.row);
            Ok(window_text(
                &WindowOf::Range { sheet: book.sheets[i].name.clone(), range: r.to_string() },
                &header,
                &rows,
            ))
        }
    }
}

/// The structure text with each table's window after its block (round 4's
/// read view A), at most `rows` data rows per table; a sheet without tables
/// shows its used range.
pub fn full_view(book: &mut Book, rows: u32) -> Result<String, String> {
    let st = model::structure(book, None)?;
    let mut out = vec![];
    if let Some(d) = &st.default_format {
        out.push(hanji_format::cellfmt::default_line(d));
        out.push(String::new());
    }
    for (i, sh) in st.sheets.iter().enumerate() {
        out.push(sheet_open(sh));
        if !sh.formats.is_empty() {
            out.push(String::new());
            out.extend(sh.formats.iter().map(|f| f.line()));
        }
        let mut any_table = false;
        for item in &sh.items {
            out.push(String::new());
            match item {
                hanji_format::sheet::SheetItem::Table(t) => {
                    any_table = true;
                    out.extend(table_lines(t));
                    out.push(String::new());
                    let k = book.table_index(&t.name).unwrap();
                    let (a, b) = book.tables[k].data_rows();
                    let rows = if b >= a && b - a + 1 > rows { Some((a, a + rows - 1)) } else { None };
                    out.push(window(book, &WindowOf::Table { name: t.name.clone(), rows })?.trim_end().to_string());
                }
                hanji_format::sheet::SheetItem::Keep(k) => out.push(format!(
                    "<keep id=\"{}\" kind=\"{}\" summary=\"{}\"/>",
                    k.id,
                    k.kind,
                    k.summary.replace('"', "&quot;")
                )),
                hanji_format::sheet::SheetItem::Chart(c) => out.push(hanji_format::sheet::chart_line(c)),
            }
        }
        if !any_table {
            if let Some(r) = &sh.range {
                let r = CellRange::parse(r).unwrap();
                let last = r.last.row.min(r.first.row + rows - 1);
                let range =
                    CellRange::new(r.first, hanji_core::cells::CellRef::new(r.last.col.min(r.first.col + 63), last));
                out.push(String::new());
                out.push(
                    window(book, &WindowOf::Range { sheet: book.sheets[i].name.clone(), range: range.to_string() })?
                        .trim_end()
                        .to_string(),
                );
            }
        }
        out.push("</sheet>".into());
        out.push(String::new());
    }
    Ok(out.join("\n"))
}
