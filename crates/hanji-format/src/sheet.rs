//! The Spreadsheet grammar (DESIGN.md §5.4): the workbook's structure as text
//! — `<sheet>` blocks holding `<table>` blocks (one pipe-table line per
//! column: name, type, number format, formula), `<chart/>` lines and
//! placeholders — and the row window through which cell data is read (a
//! pipe table whose first column is the read-only sheet row number, values
//! as displayed; §6 round 4, read view A). Cell data is written by range
//! operations (`ops.rs`), never as text.

use crate::ast::{FrontMatter, Keep};
use crate::diag::{quoted, Diagnostic};
use crate::names::Names;
use crate::parse::{chars_of, parse_tag, Parser};
use crate::serialize::{attr, front_lines};

/// Column types (§5.4, §8: typed by the source, never guessed). `mixed` is
/// a column whose cells do not share one type; each cell keeps its own.
pub const COLUMN_TYPES: &[&str] = &["text", "number", "date", "mixed"];

/// Chart types a `<chart/>` line may name.
pub const CHART_TYPES: &[&str] = &["bar", "column", "line", "pie", "area", "scatter", "doughnut", "radar"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spreadsheet {
    pub front: FrontMatter,
    pub sheets: Vec<SheetDecl>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetDecl {
    pub name: String,
    /// The used range, as the file has it (read-only; `None` for an empty sheet).
    pub range: Option<String>,
    pub items: Vec<SheetItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SheetItem {
    Table(TableDecl),
    Chart(ChartDecl),
    Keep(Keep),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableDecl {
    pub name: String,
    /// The table's area, header row first (read-only: it follows the rows).
    pub range: String,
    pub columns: Vec<ColumnDecl>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnDecl {
    pub name: String,
    pub ty: String,
    /// A number format code as the file writes it (`#,##0`, `yyyy-mm`, `@`, `General`).
    pub format: String,
    /// `=…` for a formula column, else empty.
    pub formula: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChartDecl {
    pub ty: String,
    /// Comma-separated series references (`Sales[월],Sales[이익]`).
    pub data: String,
    pub title: Option<String>,
}

impl Spreadsheet {
    pub fn tables(&self) -> impl Iterator<Item = (&SheetDecl, &TableDecl)> {
        self.sheets.iter().flat_map(|s| {
            s.items.iter().filter_map(move |i| match i {
                SheetItem::Table(t) => Some((s, t)),
                _ => None,
            })
        })
    }

    pub fn table(&self, name: &str) -> Option<(&SheetDecl, &TableDecl)> {
        self.tables().find(|(_, t)| t.name.eq_ignore_ascii_case(name))
    }

    pub fn sheet(&self, name: &str) -> Option<&SheetDecl> {
        self.sheets.iter().find(|s| s.name.eq_ignore_ascii_case(name))
    }

    pub fn keeps(&self) -> impl Iterator<Item = &Keep> {
        self.sheets.iter().flat_map(|s| {
            s.items.iter().filter_map(|i| match i {
                SheetItem::Keep(k) => Some(k),
                _ => None,
            })
        })
    }
}

/// What a number format code displays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatKind {
    General,
    Text,
    Date,
    Number,
}

/// The kind of a number format code: `@` text, a code with date or time
/// parts (`y`, `m`, `d`, `h`, `s` outside quotes and brackets) a date,
/// `General`, else a number.
pub fn format_kind(code: &str) -> FormatKind {
    if code.eq_ignore_ascii_case("general") {
        return FormatKind::General;
    }
    // The first section decides (a date code's other sections are dates too).
    let mut plain = String::new();
    let mut chars = code.chars().peekable();
    let mut elapsed = false;
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                for d in chars.by_ref() {
                    if d == '"' {
                        break;
                    }
                }
            }
            '\\' | '_' | '*' => {
                chars.next();
            }
            '[' => {
                let mut inner = String::new();
                for d in chars.by_ref() {
                    if d == ']' {
                        break;
                    }
                    inner.push(d);
                }
                let l = inner.to_ascii_lowercase();
                if !l.is_empty() && l.chars().all(|x| matches!(x, 'h' | 'm' | 's')) {
                    elapsed = true;
                }
            }
            ';' => break,
            c => plain.push(c),
        }
    }
    let l = plain.to_ascii_lowercase();
    if elapsed
        || l.contains(['y', 'd', 'h', 's'])
        || (l.contains('m') && !l.contains(['0', '#', '?']))
        || l.contains("am/pm")
    {
        return FormatKind::Date;
    }
    if l.contains('m') && l.contains(['0', '#']) && !l.contains('e') {
        // `mm:ss.0` and the like
        return FormatKind::Date;
    }
    if plain.contains('@') && !plain.contains(['0', '#', '?']) {
        return FormatKind::Text;
    }
    if plain.trim().is_empty() && code.contains('@') {
        return FormatKind::Text;
    }
    FormatKind::Number
}

/// The column type a format fits (`mixed` fits any).
pub fn type_fits_format(ty: &str, code: &str) -> bool {
    match (ty, format_kind(code)) {
        ("mixed", _) => true,
        ("text", k) => k == FormatKind::Text,
        ("date", k) => k == FormatKind::Date,
        ("number", k) => matches!(k, FormatKind::Number | FormatKind::General),
        _ => false,
    }
}

/// Why a sheet name is not one Excel accepts.
pub fn sheet_name_problem(name: &str) -> Option<String> {
    if name.trim().is_empty() {
        return Some("a sheet name is not empty".into());
    }
    if name.chars().count() > 31 {
        return Some("a sheet name has at most 31 characters".into());
    }
    if let Some(c) = name.chars().find(|c| "[]:*?/\\".contains(*c)) {
        return Some(format!("a sheet name cannot hold {c:?} (nor any of [ ] : * ? / \\)"));
    }
    if name.starts_with('\'') || name.ends_with('\'') {
        return Some("a sheet name does not begin or end with '".into());
    }
    None
}

/// Why a table name is not one Excel accepts.
pub fn table_name_problem(name: &str) -> Option<String> {
    let mut it = name.chars();
    let first = it.next()?;
    let form = "a table name is letters, digits, _ and ., starting with a letter or _";
    if !(first.is_alphabetic() || first == '_' || first == '\\')
        || !name.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '\\'))
    {
        return Some(form.into());
    }
    if name.chars().count() > 255 {
        return Some("a table name has at most 255 characters".into());
    }
    let upper = name.to_ascii_uppercase();
    let cell_like = hanji_cell_like(&upper) || upper == "R" || upper == "C";
    if cell_like {
        return Some(format!("{name} reads as a cell address; {form}, and not a cell address such as A1 or R1C1"));
    }
    None
}

fn hanji_cell_like(u: &str) -> bool {
    let k = u.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(u.len());
    let a1 = k > 0 && k <= 3 && k < u.len() && u[k..].bytes().all(|b| b.is_ascii_digit());
    let r1c1 = u.starts_with('R') && {
        let rest = u[1..].trim_start_matches(|c: char| c.is_ascii_digit());
        rest.starts_with('C') && rest[1..].bytes().all(|b| b.is_ascii_digit())
    };
    a1 || r1c1
}

const SHEET_FORM: &str = "<sheet name=\"…\"> … </sheet>";
const TABLE_FORM: &str = "<table name=\"…\" range=\"A1:F20\">, the lines | column | type | format | formula | and |---|---|---|---|, one line | name | type | format | formula | per column, and </table>";
const CHART_FORM: &str = "<chart type=\"bar\" data=\"Table[Column],Table[Column]\" title=\"…\"/>";
const KEEP_FORM: &str = "<keep id=\"…\" kind=\"…\" summary=\"…\"/>";

/// Parse the structure text of a Spreadsheet, checking placeholders
/// against `names.keeps` (a `None` list skips that check).
pub fn parse_spreadsheet(text: &str, names: &Names) -> Result<Spreadsheet, Vec<Diagnostic>> {
    let mut p = Parser::new(text, names);
    let (front, first) = p.front_matter("spreadsheet");
    let Some(front) = front.filter(|_| p.errors.is_empty()) else { return Err(p.errors) };
    let mut sheets: Vec<SheetDecl> = vec![];
    let mut keeps: Vec<(usize, usize, Keep)> = vec![];
    let n = p.lines.len();
    let mut i = first;
    let mut open: Option<(usize, SheetDecl)> = None;
    while i < n {
        let text = p.lines[i].text;
        let t = text.trim();
        if t.is_empty() {
            i += 1;
            continue;
        }
        let col = text.len() - text.trim_start().len() + 1;
        if t.starts_with("</sheet") {
            match open.take() {
                Some((_, s)) => sheets.push(s),
                None => p.err(i, col, format!("</sheet> closes no sheet; a sheet is {SHEET_FORM}.")),
            }
            i += 1;
            continue;
        }
        if t.starts_with("<sheet") {
            if let Some((at, s)) = open.take() {
                p.err(
                    at,
                    1,
                    format!("sheet \"{}\" is not closed; a sheet is {SHEET_FORM}, and sheets do not nest.", s.name),
                );
                sheets.push(s);
            }
            if let Some(s) = sheet_tag(&mut p, i) {
                open = Some((i, s));
            }
            i += 1;
            continue;
        }
        let Some((_, sheet)) = open.as_mut() else {
            p.err(i, col, format!("everything in a Spreadsheet is inside a sheet: {SHEET_FORM}."));
            i += 1;
            continue;
        };
        if t.starts_with("<table") {
            let (table, next) = table_block(&mut p, i);
            if let Some(tb) = table {
                sheet.items.push(SheetItem::Table(tb));
            }
            i = next;
            continue;
        }
        if t.starts_with("<chart") {
            if let Some(c) = chart_tag(&mut p, i) {
                sheet.items.push(SheetItem::Chart(c));
            }
            i += 1;
            continue;
        }
        if t.starts_with("<keep") {
            if let Some(k) = keep_tag(&mut p, i) {
                keeps.push((i, col, k.clone()));
                sheet.items.push(SheetItem::Keep(k));
            }
            i += 1;
            continue;
        }
        if t.starts_with("<data") {
            p.err(i, col, "a <data> block is the read view of a table's cells; cells are written by range operations, never as text. Leave it out of the structure.");
        } else if t.starts_with('|') {
            p.err(i, col, format!("a pipe table here belongs inside a table block: {TABLE_FORM}. Cell data is written by range operations."));
        } else {
            p.err(i, col, format!("a sheet holds only table blocks ({TABLE_FORM}), chart lines ({CHART_FORM}) and placeholders ({KEEP_FORM})."));
        }
        i += 1;
    }
    if let Some((at, s)) = open {
        p.err(at, 1, format!("sheet \"{}\" is not closed by a line </sheet>.", s.name));
    }
    check_names(&mut p, &sheets, &keeps);
    if !p.errors.is_empty() {
        p.errors.sort_by_key(|d| (d.line, d.col));
        return Err(p.errors);
    }
    Ok(Spreadsheet { front, sheets })
}

/// The attributes of a single-line tag at line `i`, checked against `allowed`.
fn tag_attrs(p: &mut Parser<'_>, i: usize, name: &str, form: &str, allowed: &[&str]) -> Option<Vec<(String, String)>> {
    let line = &p.lines[i];
    let lead = line.text.len() - line.text.trim_start().len();
    let src = chars_of(line, lead);
    let col = lead + 1;
    let Some((tag, end)) = parse_tag(&src, 0) else {
        p.err(i, col, format!("this line is not a well-formed tag; it is {form}."));
        return None;
    };
    if tag.name != name || tag.closing {
        p.err(i, col, format!("expected {form}."));
        return None;
    }
    if src[end..].iter().any(|s| !s.2.is_whitespace()) {
        p.err(i, src.get(end).map_or(col, |s| s.1), format!("nothing follows the tag on its line; it is {form}."));
        return None;
    }
    let mut out = vec![];
    for (k, v, c) in &tag.attrs {
        if !allowed.contains(&k.as_str()) {
            p.err(i, *c, format!("<{name}> has no attribute \"{k}\"; it is {form}."));
            return None;
        }
        if out.iter().any(|(x, _): &(String, String)| x == k) {
            p.err(i, *c, format!("\"{k}\" appears twice in <{name}>."));
            return None;
        }
        out.push((k.clone(), v.clone()));
    }
    let wants_close = name != "sheet" && name != "table";
    if wants_close != tag.self_closing {
        let msg = if wants_close {
            format!("<{name}> is a single tag closed by />: {form}.")
        } else {
            format!("<{name}> is not closed by />; it is {form}.")
        };
        p.err(i, col, msg);
        return None;
    }
    Some(out)
}

fn get<'a>(attrs: &'a [(String, String)], k: &str) -> Option<&'a str> {
    attrs.iter().find(|a| a.0 == k).map(|a| a.1.as_str())
}

fn sheet_tag(p: &mut Parser<'_>, i: usize) -> Option<SheetDecl> {
    let attrs = tag_attrs(p, i, "sheet", SHEET_FORM, &["name", "range"])?;
    let Some(name) = get(&attrs, "name") else {
        p.err(i, 1, format!("<sheet> has a name: {SHEET_FORM}."));
        return None;
    };
    if let Some(m) = sheet_name_problem(name) {
        p.err(i, 1, format!("sheet \"{name}\": {m}."));
        return None;
    }
    let range = get(&attrs, "range").map(str::to_string);
    if let Some(r) = &range {
        if !is_range(r) {
            p.err(i, 1, format!("sheet \"{name}\": range=\"{r}\" is not an A1 range such as A1:F20; it is the sheet's used range, as the file has it."));
            return None;
        }
    }
    Some(SheetDecl { name: name.to_string(), range, items: vec![] })
}

fn is_range(r: &str) -> bool {
    let cell = |s: &str| {
        let k = s.find(|c: char| !c.is_ascii_uppercase()).unwrap_or(s.len());
        k > 0 && k <= 3 && k < s.len() && s[k..].bytes().all(|b| b.is_ascii_digit()) && !s[k..].starts_with('0')
    };
    match r.split_once(':') {
        Some((a, b)) => cell(a) && cell(b),
        None => cell(r),
    }
}

/// Split a pipe-table line into cells (`\|` is a literal pipe). A cell
/// loses one space on each side (the separator canonical form writes), so
/// a name's own leading or trailing spaces survive.
pub fn split_pipe_row(line: &str) -> Option<Vec<String>> {
    let t = line.trim();
    let inner = t.strip_prefix('|')?.strip_suffix('|').filter(|_| t.len() >= 2)?;
    let mut cells = vec![];
    let mut cur = String::new();
    let mut it = inner.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\\' if it.peek() == Some(&'|') => {
                cur.push('|');
                it.next();
            }
            '|' => cells.push(unpad(&std::mem::take(&mut cur))),
            c => cur.push(c),
        }
    }
    cells.push(unpad(&cur));
    Some(cells)
}

fn unpad(s: &str) -> String {
    if s.trim().is_empty() {
        return String::new();
    }
    let s = s.strip_prefix(' ').unwrap_or(s);
    s.strip_suffix(' ').unwrap_or(s).to_string()
}

fn is_delim_row(cells: &[String]) -> bool {
    !cells.is_empty() && cells.iter().all(|c| c.trim().chars().all(|x| x == '-' || x == ':') && c.contains('-'))
}

fn table_block(p: &mut Parser<'_>, i: usize) -> (Option<TableDecl>, usize) {
    let n = p.lines.len();
    let close = (i + 1..n).find(|&k| {
        let t = p.lines[k].text.trim();
        t.starts_with("</table") || t.starts_with("<table") || t.starts_with("<sheet") || t.starts_with("</sheet")
    });
    let next = match close {
        Some(k) if p.lines[k].text.trim().starts_with("</table") => k + 1,
        Some(k) => {
            p.err(i, 1, format!("this table is not closed by a line </table>; a table is {TABLE_FORM}."));
            return (None, k);
        }
        None => {
            p.err(i, 1, format!("this table is not closed by a line </table>; a table is {TABLE_FORM}."));
            return (None, n);
        }
    };
    let close = next - 1;
    let closer = p.lines[close].text.trim();
    if closer != "</table>" {
        p.err(close, 1, "the line closing a table is exactly </table>.");
    }
    let Some(attrs) = tag_attrs(p, i, "table", TABLE_FORM, &["name", "range"]) else { return (None, next) };
    let (Some(name), Some(range)) = (get(&attrs, "name"), get(&attrs, "range")) else {
        p.err(i, 1, format!("<table> has a name and a range: {TABLE_FORM}."));
        return (None, next);
    };
    if let Some(m) = table_name_problem(name) {
        p.err(i, 1, format!("table \"{name}\": {m}."));
        return (None, next);
    }
    if !is_range(range) {
        p.err(i, 1, format!("table \"{name}\": range=\"{range}\" is not an A1 range such as A1:F20."));
        return (None, next);
    }
    let body: Vec<usize> = (i + 1..close).filter(|&k| !p.lines[k].text.trim().is_empty()).collect();
    let header = body.first().and_then(|&k| split_pipe_row(p.lines[k].text));
    let head_ok = header.as_ref().is_some_and(|h| {
        let h: Vec<String> = h.iter().map(|c| c.trim().to_ascii_lowercase()).collect();
        h == ["column", "type", "format", "formula"]
    });
    if !head_ok {
        let at = body.first().copied().unwrap_or(i);
        p.err(
            at,
            1,
            "a table block begins with the line | column | type | format | formula | and then |---|---|---|---|.",
        );
        return (None, next);
    }
    if !body.get(1).and_then(|&k| split_pipe_row(p.lines[k].text)).is_some_and(|d| is_delim_row(&d)) {
        let at = body.get(1).copied().unwrap_or(body[0]);
        p.err(at, 1, "the second line of a table block is |---|---|---|---|.");
        return (None, next);
    }
    let mut columns: Vec<ColumnDecl> = vec![];
    let before = p.errors.len();
    for &k in &body[2..] {
        let Some(cells) = split_pipe_row(p.lines[k].text) else {
            p.err(k, 1, "each line of a table block is one column: | name | type | format | formula |.");
            continue;
        };
        if cells.len() < 3 || cells.len() > 4 {
            p.err(k, 1, format!("a column line has four cells — | name | type | format | formula | — the formula cell empty for a column that is not a formula column; this line has {}.", cells.len()));
            continue;
        }
        let (cname, ty, format) = (&cells[0], &cells[1].trim().to_string(), &cells[2]);
        let formula = cells.get(3).map(|f| f.trim().to_string()).unwrap_or_default();
        if cname.trim().is_empty() {
            p.err(k, 1, "a column has a name (its header cell).");
            continue;
        }
        if !COLUMN_TYPES.contains(&ty.as_str()) {
            p.err(k, 1, format!("column \"{cname}\": the type is one of {}, not \"{ty}\".", quoted_list(COLUMN_TYPES)));
            continue;
        }
        if format.trim().is_empty() {
            p.err(k, 1, format!("column \"{cname}\" has a number format (General for none; @ for text)."));
            continue;
        }
        if !formula.is_empty() && !formula.starts_with('=') {
            p.err(k, 1, format!("column \"{cname}\": a formula starts with \"=\"; the formula cell is empty for a column that is not a formula column."));
            continue;
        }
        if columns.iter().any(|c| c.name.eq_ignore_ascii_case(cname)) {
            p.err(
                k,
                1,
                format!("table \"{name}\" has two columns named \"{cname}\"; column names are unique in a table."),
            );
            continue;
        }
        columns.push(ColumnDecl { name: cname.clone(), ty: ty.clone(), format: format.clone(), formula });
    }
    if columns.is_empty() && p.errors.len() == before {
        p.err(i, 1, format!("table \"{name}\" has no columns; a table has one line per column."));
    }
    ((p.errors.len() == before).then(|| TableDecl { name: name.to_string(), range: range.to_string(), columns }), next)
}

fn quoted_list(v: &[&str]) -> String {
    quoted(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn chart_tag(p: &mut Parser<'_>, i: usize) -> Option<ChartDecl> {
    let attrs = tag_attrs(p, i, "chart", CHART_FORM, &["type", "data", "title"])?;
    let (Some(ty), Some(data)) = (get(&attrs, "type"), get(&attrs, "data")) else {
        p.err(i, 1, format!("<chart> has a type and data: {CHART_FORM}."));
        return None;
    };
    if !CHART_TYPES.contains(&ty) {
        p.err(i, 1, format!("chart type \"{ty}\" is not one of {}.", quoted_list(CHART_TYPES)));
        return None;
    }
    if data.split(',').any(|s| s.trim().is_empty()) {
        p.err(
            i,
            1,
            format!("chart data is a comma-separated list of columns such as Sales[월],Sales[이익]: {CHART_FORM}."),
        );
        return None;
    }
    Some(ChartDecl { ty: ty.into(), data: data.into(), title: get(&attrs, "title").map(str::to_string) })
}

fn keep_tag(p: &mut Parser<'_>, i: usize) -> Option<Keep> {
    let attrs = tag_attrs(p, i, "keep", KEEP_FORM, &["id", "kind", "summary"])?;
    match (get(&attrs, "id"), get(&attrs, "kind"), get(&attrs, "summary")) {
        (Some(id), Some(kind), Some(summary)) => {
            Some(Keep { id: id.into(), kind: kind.into(), summary: summary.into() })
        }
        _ => {
            p.err(i, 1, format!("a placeholder keeps its id, kind and summary as they are in the file: {KEEP_FORM}."));
            None
        }
    }
}

fn check_names(p: &mut Parser<'_>, sheets: &[SheetDecl], keeps: &[(usize, usize, Keep)]) {
    let line_of = |p: &Parser<'_>, pat: &str| (0..p.lines.len()).find(|&k| p.lines[k].text.contains(pat)).unwrap_or(0);
    for (k, s) in sheets.iter().enumerate() {
        if sheets[..k].iter().any(|o| o.name.to_lowercase() == s.name.to_lowercase()) {
            let at = line_of(p, &format!("name=\"{}\"", s.name));
            p.err(at, 1, format!("two sheets are named \"{}\"; sheet names are unique (case does not count).", s.name));
        }
    }
    let tables: Vec<&TableDecl> = sheets
        .iter()
        .flat_map(|s| s.items.iter().filter_map(|i| if let SheetItem::Table(t) = i { Some(t) } else { None }))
        .collect();
    for (k, t) in tables.iter().enumerate() {
        if tables[..k].iter().any(|o| o.name.to_lowercase() == t.name.to_lowercase()) {
            let at = line_of(p, &format!("name=\"{}\"", t.name));
            p.err(at, 1, format!("two tables are named \"{}\"; table names are unique in the workbook.", t.name));
        }
    }
    let known = p.names.keeps.clone();
    for (k, (line, col, keep)) in keeps.iter().enumerate() {
        if keeps[..k].iter().any(|o| o.2.id == keep.id) {
            p.err(*line, *col, format!("placeholder id=\"{}\" appears twice; keep each placeholder once.", keep.id));
        }
        if let Some(known) = &known {
            match known.iter().find(|x| x.id == keep.id) {
                None => p.err(*line, *col, format!("placeholder id=\"{}\" is not in this file. Placeholders come from the file: keep or delete them, but never create one.", keep.id)),
                Some(x) if x != keep => p.err(
                    *line,
                    *col,
                    format!("placeholder id=\"{}\" was altered; keep it exactly as <keep id=\"{}\" kind=\"{}\" summary=\"{}\"/>.", x.id, x.id, x.kind, attr(&x.summary)),
                ),
                _ => {}
            }
        }
    }
}

/// Canonical text of a Spreadsheet (§5.1): fixed attribute order, a blank
/// line between the items of a sheet and between sheets, no table padding.
pub fn serialize_spreadsheet(s: &Spreadsheet) -> String {
    let mut out = front_lines(&s.front);
    for sh in &s.sheets {
        out.push(String::new());
        out.push(sheet_open(sh));
        for item in &sh.items {
            out.push(String::new());
            match item {
                SheetItem::Table(t) => out.extend(table_lines(t)),
                SheetItem::Chart(c) => out.push(chart_line(c)),
                SheetItem::Keep(k) => out.push(format!(
                    "<keep id=\"{}\" kind=\"{}\" summary=\"{}\"/>",
                    attr(&k.id),
                    attr(&k.kind),
                    attr(&k.summary)
                )),
            }
        }
        out.push("</sheet>".into());
    }
    out.join("\n") + "\n"
}

pub fn sheet_open(sh: &SheetDecl) -> String {
    match &sh.range {
        Some(r) => format!("<sheet name=\"{}\" range=\"{r}\">", attr(&sh.name)),
        None => format!("<sheet name=\"{}\">", attr(&sh.name)),
    }
}

/// A pipe-table cell as written: `|` escaped, line breaks as `<br/>`.
pub fn pipe_cell(s: &str) -> String {
    s.replace('|', "\\|").replace("\r\n", "<br/>").replace(['\n', '\r'], "<br/>")
}

fn row_line(cells: &[String]) -> String {
    format!("| {} |", cells.join(" | "))
}

pub fn table_lines(t: &TableDecl) -> Vec<String> {
    let mut out = vec![
        format!("<table name=\"{}\" range=\"{}\">", attr(&t.name), t.range),
        "| column | type | format | formula |".into(),
        "|---|---|---|---|".into(),
    ];
    for c in &t.columns {
        out.push(row_line(&[pipe_cell(&c.name), c.ty.clone(), pipe_cell(&c.format), pipe_cell(&c.formula)]));
    }
    out.push("</table>".into());
    out
}

pub fn chart_line(c: &ChartDecl) -> String {
    let mut s = format!("<chart type=\"{}\" data=\"{}\"", attr(&c.ty), attr(&c.data));
    if let Some(t) = &c.title {
        s.push_str(&format!(" title=\"{}\"", attr(t)));
    }
    s + "/>"
}

/// What a row window shows: a table's rows, or a range of a sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowOf {
    /// `rows`: the sheet rows shown when not every data row is (`2:101`).
    Table { name: String, rows: Option<(u32, u32)> },
    /// A range of a sheet (`A1:F50`); its columns are named by their letters.
    Range { sheet: String, range: String },
}

/// The row window (§5.4, round 4 read view A): `<data …>`, the header row
/// `| row | … |`, a delimiter, one line per sheet row (blank rows included),
/// `</data>`. `rows` are `(sheet row, displayed values)`.
pub fn window_text(of: &WindowOf, header: &[String], rows: &[(u32, Vec<String>)]) -> String {
    let open = match of {
        WindowOf::Table { name, rows: None } => format!("<data table=\"{}\">", attr(name)),
        WindowOf::Table { name, rows: Some((a, b)) } => format!("<data table=\"{}\" rows=\"{a}:{b}\">", attr(name)),
        WindowOf::Range { sheet, range } => format!("<data sheet=\"{}\" range=\"{range}\">", attr(sheet)),
    };
    let mut out = String::with_capacity(64 * (rows.len() + 3));
    out.push_str(&open);
    out.push('\n');
    let mut head = vec!["row".to_string()];
    head.extend(header.iter().map(|h| pipe_cell(h)));
    out.push_str(&row_line(&head));
    out.push('\n');
    out.push('|');
    for _ in 0..head.len() {
        out.push_str("---|");
    }
    out.push('\n');
    for (r, cells) in rows {
        let mut line = vec![r.to_string()];
        line.extend(cells.iter().map(|c| pipe_cell(c)));
        out.push_str(&row_line(&line));
        out.push('\n');
    }
    out.push_str("</data>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "---\ntype: spreadsheet\nformat: xlsx\nschema: 1\n---\n\n<sheet name=\"매출\" range=\"A1:E1201\">\n\n<table name=\"Sales\" range=\"A1:E1201\">\n| column | type | format | formula |\n|---|---|---|---|\n| 월 | date | yyyy-mm |  |\n| 매출 | number | #,##0 |  |\n| 이익 | number | #,##0 | =[@매출]-[@원가] |\n</table>\n\n<chart type=\"bar\" data=\"Sales[월],Sales[이익]\" title=\"월별 이익\"/>\n\n<keep id=\"k1\" kind=\"merged-cells\" summary=\"G1:H1\"/>\n</sheet>\n";

    #[test]
    fn round_trip() {
        let s = parse_spreadsheet(TEXT, &Names::default()).unwrap();
        assert_eq!(s.sheets.len(), 1);
        let (_, t) = s.table("sales").unwrap();
        assert_eq!(t.columns[2].formula, "=[@매출]-[@원가]");
        assert_eq!(serialize_spreadsheet(&s), TEXT);
        // The §5.4 example, without blank lines, parses to the same thing.
        let tight = TEXT.replace("\n\n<", "\n<");
        assert_eq!(parse_spreadsheet(&tight, &Names::default()).unwrap(), s);
    }

    fn errs(text: &str) -> Vec<String> {
        let k1 = Keep { id: "k1".into(), kind: "merged-cells".into(), summary: "G1:H1".into() };
        parse_spreadsheet(text, &Names { keeps: Some(vec![k1]), ..Default::default() })
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|d| d.to_string())
            .collect()
    }

    #[test]
    fn errors_for_the_model() {
        let e = errs(&TEXT.replace("| 월 | date |", "| 월 | month |"));
        assert!(e[0].contains("line 12") && e[0].contains("\"text\", \"number\", \"date\", \"mixed\""), "{e:?}");
        let e = errs(&TEXT.replace("</table>\n", ""));
        assert!(e[0].contains("not closed by a line </table>"), "{e:?}");
        let e = errs(&TEXT.replace(" title=", " colour=\"red\" title="));
        assert!(e[0].contains("no attribute \"colour\""), "{e:?}");
        let e = errs(&TEXT.replace("id=\"k1\"", "id=\"k2\""));
        assert!(e[0].contains("placeholder id=\"k2\" is not in this file"), "{e:?}");
        let e = errs(&TEXT.replace("G1:H1", "G1:H2"));
        assert!(e[0].contains("was altered"), "{e:?}");
        let e = errs(&TEXT.replace("name=\"Sales\"", "name=\"A1\""));
        assert!(e[0].contains("reads as a cell address"), "{e:?}");
        let e =
            errs(&TEXT.replace("| 이익 | number | #,##0 | =[@매출]-[@원가] |", "| 이익 | number | #,##0 | [@매출] |"));
        assert!(e[0].contains("a formula starts with \"=\""), "{e:?}");
        let e = errs(&TEXT.replace("</sheet>\n", "| 2 | 2026-01 | 1 | 2 |\n</sheet>\n"));
        assert!(e[0].contains("range operations"), "{e:?}");
    }

    #[test]
    fn format_kinds() {
        use FormatKind::*;
        for (c, k) in [
            ("General", General),
            ("@", Text),
            ("#,##0", Number),
            ("0.0%", Number),
            ("00000", Number),
            ("yyyy-mm", Date),
            ("m/d/yy", Date),
            ("[$-412]yyyy\"년\" m\"월\"", Date),
            ("h:mm AM/PM", Date),
            ("mm:ss.0", Date),
            ("[h]:mm", Date),
            ("#,##0 \"m²\"", Number),
            ("[Red]#,##0;[Blue]-#,##0", Number),
            ("_-* #,##0_-;-* #,##0_-;_-* \"-\"_-;_-@_-", Number),
            ("0.00E+00", Number),
        ] {
            assert_eq!(format_kind(c), k, "{c}");
        }
    }

    #[test]
    fn window() {
        let w = window_text(
            &WindowOf::Table { name: "Sales".into(), rows: None },
            &["월".into(), "매출".into()],
            &[(2, vec!["2026-01".into(), "12,000,000".into()]), (3, vec![String::new(), String::new()])],
        );
        assert_eq!(w, "<data table=\"Sales\">\n| row | 월 | 매출 |\n|---|---|---|\n| 2 | 2026-01 | 12,000,000 |\n| 3 |  |  |\n</data>\n");
        assert_eq!(split_pipe_row("| a\\|b | c |").unwrap(), ["a|b", "c"]);
    }
}
