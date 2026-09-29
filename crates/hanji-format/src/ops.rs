//! Range operations (DESIGN.md §5.4, §6 round 4 write shape A): the JSON
//! list of operations from a closed set of ten through which a model writes
//! cell data, applied in order. [`parse_ops`] reads the list and checks each
//! operation's shape; [`check_ops`] checks names, types, formats and
//! formulas against the workbook's structure, following the list as it
//! changes the structure. Errors are written for the model: the operation's
//! number, the field, the expected form and the allowed names.

use std::fmt;

use serde_json::Value as Json;

use crate::diag::quoted;
use crate::formula::{self, FormulaProblem};
use crate::sheet::{format_kind, sheet_name_problem, table_name_problem, type_fits_format, FormatKind, Spreadsheet};
use crate::sheet::{ColumnDecl, SheetDecl, SheetItem, TableDecl};

/// The ten operations, with their required and optional keys.
pub const OPS: &[(&str, &[&str], &[&str])] = &[
    ("set", &["range", "values"], &[]),
    ("append_rows", &["table", "rows"], &[]),
    ("insert_rows", &["table", "before", "rows"], &[]),
    ("delete_rows", &["table", "rows"], &[]),
    ("fill_formula", &["table", "column", "formula"], &[]),
    ("set_type", &["table", "column", "type"], &["format"]),
    ("add_column", &["table", "column"], &[]),
    ("sort", &["table", "keys"], &[]),
    ("add_table", &["sheet", "name", "anchor", "columns"], &["rows"]),
    ("add_sheet", &["name"], &[]),
];

/// No answer adds more rows than this (§5.4: the model never types bulk rows).
pub const MAX_NEW_ROWS: usize = 50;

/// A cell value as JSON writes it: text, a number, `true`/`false`, or null
/// (empty). A date is text `YYYY-MM-DD` (`YYYY-MM` for the first of the
/// month), read as a date only in a date column.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    Text(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Number(n) => write!(f, "{n}"),
            Value::Text(s) => write!(f, "{s:?}"),
        }
    }
}

/// A row: `(column, value)` in the order written.
pub type Row = Vec<(String, Value)>;

#[derive(Clone, Debug, PartialEq)]
pub struct ColumnSpec {
    pub name: String,
    pub ty: String,
    pub format: Option<String>,
    pub formula: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortKey {
    pub column: String,
    pub desc: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RangeOp {
    Set { range: String, values: Vec<Vec<Value>> },
    AppendRows { table: String, rows: Vec<Row> },
    InsertRows { table: String, before: u32, rows: Vec<Row> },
    DeleteRows { table: String, first: u32, last: u32 },
    FillFormula { table: String, column: String, formula: String },
    SetType { table: String, column: String, ty: String, format: Option<String> },
    AddColumn { table: String, column: ColumnSpec },
    Sort { table: String, keys: Vec<SortKey> },
    AddTable { sheet: String, name: String, anchor: String, columns: Vec<ColumnSpec>, rows: Vec<Row> },
    AddSheet { name: String },
}

impl RangeOp {
    pub fn name(&self) -> &'static str {
        match self {
            RangeOp::Set { .. } => "set",
            RangeOp::AppendRows { .. } => "append_rows",
            RangeOp::InsertRows { .. } => "insert_rows",
            RangeOp::DeleteRows { .. } => "delete_rows",
            RangeOp::FillFormula { .. } => "fill_formula",
            RangeOp::SetType { .. } => "set_type",
            RangeOp::AddColumn { .. } => "add_column",
            RangeOp::Sort { .. } => "sort",
            RangeOp::AddTable { .. } => "add_table",
            RangeOp::AddSheet { .. } => "add_sheet",
        }
    }

    /// Rows the operation adds.
    pub fn new_rows(&self) -> usize {
        match self {
            RangeOp::AppendRows { rows, .. } | RangeOp::InsertRows { rows, .. } | RangeOp::AddTable { rows, .. } => {
                rows.len()
            }
            _ => 0,
        }
    }
}

/// A problem with one operation (`op` 1-based; 0 for the list as a whole).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpError {
    pub op: usize,
    /// The operation's name, when it has a known one.
    pub name: Option<String>,
    /// The key the problem is in (`rows`, `values`, `column`).
    pub field: Option<String>,
    pub message: String,
}

impl fmt::Display for OpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.op == 0 {
            return f.write_str(&self.message);
        }
        write!(f, "operation {}", self.op)?;
        if let Some(n) = &self.name {
            write!(f, " ({n})")?;
        }
        if let Some(k) = &self.field {
            write!(f, ", \"{k}\"")?;
        }
        write!(f, ": {}", self.message)
    }
}

impl std::error::Error for OpError {}

/// All errors, one per line.
pub fn render_errors(errs: &[OpError]) -> String {
    errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
}

fn op_list() -> String {
    OPS.iter().map(|o| o.0).collect::<Vec<_>>().join(", ")
}

/// Read the JSON list (a JSON string holding the list, or a fenced code
/// block, is accepted too) and check every operation's keys and value shapes.
pub fn parse_ops(text: &str) -> Result<Vec<RangeOp>, Vec<OpError>> {
    let whole = |m: String| vec![OpError { op: 0, name: None, field: None, message: m }];
    let mut s = text.trim();
    if let Some(rest) = s.strip_prefix("```") {
        let body = rest.split_once('\n').map_or("", |x| x.1);
        s = body.trim_end().strip_suffix("```").unwrap_or(body).trim();
    }
    let mut v: Json = serde_json::from_str(s).map_err(|e| {
        whole(format!("the operations are not JSON ({e}); they are a JSON list such as [{{\"op\": \"set\", \"range\": \"Sheet1!B7\", \"values\": [[1204]]}}]."))
    })?;
    if let Json::String(inner) = &v {
        v = serde_json::from_str(inner)
            .map_err(|e| whole(format!("the operations are a JSON list, not a string ({e}).")))?;
    }
    let Json::Array(items) = v else {
        return Err(whole("the operations are a JSON list: [{\"op\": …}, …].".into()));
    };
    let mut ops = vec![];
    let mut errs = vec![];
    for (k, item) in items.iter().enumerate() {
        match one_op(k + 1, item) {
            Ok(op) => ops.push(op),
            Err(e) => errs.push(e),
        }
    }
    let added: usize = ops.iter().map(RangeOp::new_rows).sum();
    if added > MAX_NEW_ROWS {
        errs.push(OpError {
            op: 0,
            name: None,
            field: None,
            message: format!("these operations add {added} rows; no answer adds more than {MAX_NEW_ROWS} rows in total. Bulk rows come from an import, not from rows typed or generated."),
        });
    }
    if errs.is_empty() {
        Ok(ops)
    } else {
        Err(errs)
    }
}

struct Ctx {
    op: usize,
    name: &'static str,
}

impl Ctx {
    fn err(&self, field: &str, m: impl Into<String>) -> OpError {
        OpError {
            op: self.op,
            name: Some(self.name.into()),
            field: (!field.is_empty()).then(|| field.to_string()),
            message: m.into(),
        }
    }
}

fn one_op(k: usize, item: &Json) -> Result<RangeOp, OpError> {
    let bad = |m: String| OpError { op: k, name: None, field: None, message: m };
    let Json::Object(o) = item else {
        return Err(bad(format!("an operation is an object {{\"op\": …}}; \"op\" is one of {}.", op_list())));
    };
    let Some(Json::String(name)) = o.get("op") else {
        return Err(bad(format!("\"op\" is one of {}.", op_list())));
    };
    let Some(&(name, req, opt)) = OPS.iter().find(|x| x.0 == name) else {
        return Err(bad(format!("\"{name}\" is not an operation; \"op\" is one of {}.", op_list())));
    };
    let cx = Ctx { op: k, name };
    let missing: Vec<&str> = req.iter().copied().filter(|a| !o.contains_key(*a)).collect();
    let extra: Vec<&str> =
        o.keys().map(String::as_str).filter(|a| *a != "op" && !req.contains(a) && !opt.contains(a)).collect();
    if !missing.is_empty() || !extra.is_empty() {
        let takes = format!(
            "it takes {}{}",
            req.join(", "),
            if opt.is_empty() { String::new() } else { format!(" and optionally {}", opt.join(", ")) }
        );
        let what = if !missing.is_empty() {
            format!("missing {}", missing.join(", "))
        } else {
            format!("unknown {}", extra.join(", "))
        };
        return Err(cx.err("", format!("{takes}; {what}.")));
    }
    let s = |key: &str| -> Result<String, OpError> {
        match o.get(key) {
            Some(Json::String(v)) if !v.trim().is_empty() => Ok(v.trim().to_string()),
            _ => Err(cx.err(key, "is a non-empty string.")),
        }
    };
    let opt_s = |key: &str| -> Result<Option<String>, OpError> {
        match o.get(key) {
            None | Some(Json::Null) => Ok(None),
            Some(Json::String(v)) if v.trim().is_empty() => Ok(None),
            Some(Json::String(v)) => Ok(Some(v.trim().to_string())),
            _ => Err(cx.err(key, "is a string.")),
        }
    };
    Ok(match name {
        "set" => {
            let range = s("range")?;
            let Some(Json::Array(rows)) = o.get("values") else {
                return Err(cx.err("values", "is a list of rows, each a list of values: [[1204]] for one cell."));
            };
            let mut values = vec![];
            for r in rows {
                let Json::Array(cells) = r else {
                    return Err(cx.err("values", "is a list of rows, each a list of values: [[1204]] for one cell, [[1, 2], [3, 4]] for two rows of two."));
                };
                values.push(cells.iter().map(|c| value(&cx, "values", c)).collect::<Result<Vec<_>, _>>()?);
            }
            RangeOp::Set { range, values }
        }
        "append_rows" => RangeOp::AppendRows { table: s("table")?, rows: rows(&cx, o.get("rows"))? },
        "insert_rows" => {
            let before = row_number(&cx, "before", o.get("before"))?;
            RangeOp::InsertRows { table: s("table")?, before, rows: rows(&cx, o.get("rows"))? }
        }
        "delete_rows" => {
            let (first, last) = match o.get("rows") {
                Some(Json::Number(n)) if n.as_u64().is_some_and(|n| n >= 1) => {
                    let n = n.as_u64().unwrap() as u32;
                    (n, n)
                }
                Some(Json::String(v)) => parse_rows(v)
                    .ok_or_else(|| cx.err("rows", format!("is \"57\" or \"57:58\" (sheet row numbers), not {v:?}.")))?,
                _ => return Err(cx.err("rows", "is \"57\" or \"57:58\" (sheet row numbers).")),
            };
            if last < first {
                return Err(cx.err("rows", format!("\"{first}:{last}\" runs backwards; write \"{last}:{first}\".")));
            }
            RangeOp::DeleteRows { table: s("table")?, first, last }
        }
        "fill_formula" => RangeOp::FillFormula { table: s("table")?, column: s("column")?, formula: s("formula")? },
        "set_type" => {
            RangeOp::SetType { table: s("table")?, column: s("column")?, ty: s("type")?, format: opt_s("format")? }
        }
        "add_column" => RangeOp::AddColumn { table: s("table")?, column: column_spec(&cx, "column", o.get("column"))? },
        "sort" => RangeOp::Sort { table: s("table")?, keys: sort_keys(&cx, o.get("keys"))? },
        "add_table" => {
            let Some(Json::Array(cols)) = o.get("columns").filter(|c| c.as_array().is_some_and(|a| !a.is_empty()))
            else {
                return Err(
                    cx.err("columns", "is a non-empty list of {\"name\", \"type\", \"format\"[, \"formula\"]}.")
                );
            };
            let columns = cols.iter().map(|c| column_spec(&cx, "columns", Some(c))).collect::<Result<Vec<_>, _>>()?;
            let rows = match o.get("rows") {
                None | Some(Json::Null) => vec![],
                r => rows(&cx, r)?,
            };
            RangeOp::AddTable { sheet: s("sheet")?, name: s("name")?, anchor: s("anchor")?, columns, rows }
        }
        _ => RangeOp::AddSheet { name: s("name")? },
    })
}

fn parse_rows(v: &str) -> Option<(u32, u32)> {
    let num = |s: &str| s.trim().parse::<u32>().ok().filter(|n| *n >= 1);
    match v.split_once(':') {
        Some((a, b)) => Some((num(a)?, num(b)?)),
        None => num(v).map(|n| (n, n)),
    }
}

fn row_number(cx: &Ctx, field: &str, v: Option<&Json>) -> Result<u32, OpError> {
    match v {
        Some(Json::Number(n)) if n.as_u64().is_some_and(|n| (1..=1_048_576).contains(&n)) => {
            Ok(n.as_u64().unwrap() as u32)
        }
        _ => Err(cx.err(field, "is a sheet row number such as 19.")),
    }
}

fn value(cx: &Ctx, field: &str, v: &Json) -> Result<Value, OpError> {
    Ok(match v {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => {
            Value::Number(n.as_f64().ok_or_else(|| cx.err(field, format!("{n} is not a number a cell can hold.")))?)
        }
        Json::String(s) => Value::Text(s.clone()),
        other => {
            return Err(cx.err(
                field,
                format!(
                    "{other} is not a cell value; a value is text (a string), a number, a date \"YYYY-MM-DD\" or null."
                ),
            ))
        }
    })
}

fn rows(cx: &Ctx, v: Option<&Json>) -> Result<Vec<Row>, OpError> {
    let Some(Json::Array(items)) = v else {
        return Err(cx.err("rows", "is a list of rows, each an object {column: value}."));
    };
    items
        .iter()
        .enumerate()
        .map(|(k, r)| {
            let Json::Object(o) = r else {
                return Err(cx.err("rows", format!("row {} is an object {{column: value}}, not {r}.", k + 1)));
            };
            o.iter().map(|(c, v)| Ok((c.clone(), value(cx, "rows", v)?))).collect()
        })
        .collect()
}

fn column_spec(cx: &Ctx, field: &str, v: Option<&Json>) -> Result<ColumnSpec, OpError> {
    let form = "{\"name\": …, \"type\": \"text\" | \"number\" | \"date\", \"format\": …[, \"formula\": \"=…\"]}";
    let Some(Json::Object(o)) = v else { return Err(cx.err(field, format!("a column is {form}."))) };
    if let Some(k) = o.keys().find(|k| !["name", "type", "format", "formula"].contains(&k.as_str())) {
        return Err(cx.err(field, format!("a column has no key \"{k}\"; it is {form}.")));
    }
    let get = |k: &str| match o.get(k) {
        Some(Json::String(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
        _ => None,
    };
    let (Some(name), Some(ty)) = (get("name"), get("type")) else {
        return Err(cx.err(field, format!("a column is {form}; it needs a name and a type.")));
    };
    Ok(ColumnSpec { name, ty, format: get("format"), formula: get("formula") })
}

fn sort_keys(cx: &Ctx, v: Option<&Json>) -> Result<Vec<SortKey>, OpError> {
    let form = "is a non-empty list of {\"column\": name, \"order\": \"asc\" | \"desc\"}.";
    let Some(Json::Array(items)) = v.filter(|v| v.as_array().is_some_and(|a| !a.is_empty())) else {
        return Err(cx.err("keys", form));
    };
    items
        .iter()
        .map(|k| {
            let (col, order) = match k {
                Json::String(c) => (Some(c.clone()), "asc".to_string()),
                Json::Object(o) => (
                    o.get("column").and_then(Json::as_str).map(str::to_string),
                    o.get("order").and_then(Json::as_str).unwrap_or("asc").to_ascii_lowercase(),
                ),
                _ => (None, String::new()),
            };
            match (col, order.as_str()) {
                (Some(column), "asc" | "desc") => Ok(SortKey { column, desc: order == "desc" }),
                _ => Err(cx.err("keys", form)),
            }
        })
        .collect()
}

/// A cell address `Sheet!D70` / `'My sheet'!D70:E71` split into sheet and cells.
pub fn split_range(range: &str) -> Option<(String, String)> {
    let (sheet, cells) = range.rsplit_once('!')?;
    let sheet = sheet.trim();
    let sheet = match sheet.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        Some(q) => q.replace("''", "'"),
        None => sheet.to_string(),
    };
    (!sheet.is_empty()).then(|| (sheet, cells.trim().replace('$', "").to_ascii_uppercase()))
}

/// `(col, row)` of `D70` (0-based column).
fn cell(s: &str) -> Option<(u32, u32)> {
    let k = s.find(|c: char| !c.is_ascii_alphabetic())?;
    let (l, d) = s.split_at(k);
    if l.is_empty() || l.len() > 3 || d.is_empty() || !d.bytes().all(|b| b.is_ascii_digit()) || d.starts_with('0') {
        return None;
    }
    let mut c: u32 = 0;
    for b in l.bytes() {
        c = c * 26 + (b.to_ascii_uppercase() - b'A' + 1) as u32;
    }
    let r: u32 = d.parse().ok()?;
    (c <= 16_384 && r <= 1_048_576).then(|| (c - 1, r))
}

/// `(c0, r0, c1, r1)` of `D70` or `D70:E71`.
pub fn area(s: &str) -> Option<(u32, u32, u32, u32)> {
    match s.split_once(':') {
        Some((a, b)) => {
            let (a, b) = (cell(a)?, cell(b)?);
            Some((a.0, a.1, b.0, b.1))
        }
        None => cell(s).map(|a| (a.0, a.1, a.0, a.1)),
    }
}

/// Number formats a column may take: the file's own and the built-in ones.
#[derive(Clone, Debug, Default)]
pub struct Formats(pub Vec<String>);

impl Formats {
    fn allowed(&self, f: &str) -> bool {
        self.0.is_empty() || self.0.iter().any(|x| x == f)
    }
    fn list(&self) -> String {
        let shown: Vec<String> = self.0.iter().take(40).cloned().collect();
        let more = if self.0.len() > 40 { format!(" (and {} more)", self.0.len() - 40) } else { String::new() };
        format!("{}{more}", quoted(&shown))
    }
}

/// Check names, types, formats and formulas of `ops` against `structure`,
/// following the list as it adds sheets, tables and columns and moves rows.
/// `formats` lists the number formats the file allows (empty: any).
pub fn check_ops(ops: &[RangeOp], structure: &Spreadsheet, formats: &Formats) -> Result<(), Vec<OpError>> {
    let mut st = structure.clone();
    let mut errs = vec![];
    for (k, op) in ops.iter().enumerate() {
        let cx = Ctx { op: k + 1, name: op.name() };
        if let Err(e) = check_one(&cx, op, &mut st, formats) {
            errs.push(e);
            // Later operations depend on this one; one error per list is clearer than a cascade.
            break;
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

fn names_of<'a>(it: impl Iterator<Item = &'a str>) -> String {
    quoted(&it.map(str::to_string).collect::<Vec<_>>())
}

fn find_table(cx: &Ctx, st: &mut Spreadsheet, name: &str) -> Result<(usize, usize), OpError> {
    for (si, s) in st.sheets.iter().enumerate() {
        for (ti, it) in s.items.iter().enumerate() {
            if let SheetItem::Table(t) = it {
                if t.name.eq_ignore_ascii_case(name) {
                    return Ok((si, ti));
                }
            }
        }
    }
    let all = names_of(st.tables().map(|(_, t)| t.name.as_str()));
    let hint = if all.is_empty() {
        "The workbook has no tables; add one with add_table.".to_string()
    } else {
        format!("Tables: {all}.")
    };
    Err(cx.err("table", format!("there is no table \"{name}\". {hint}")))
}

fn table_mut(st: &mut Spreadsheet, (si, ti): (usize, usize)) -> &mut TableDecl {
    match &mut st.sheets[si].items[ti] {
        SheetItem::Table(t) => t,
        _ => unreachable!(),
    }
}

fn column_index(cx: &Ctx, field: &str, t: &TableDecl, name: &str) -> Result<usize, OpError> {
    t.columns.iter().position(|c| c.name == name).ok_or_else(|| {
        cx.err(
            field,
            format!(
                "table {} has no column \"{name}\" (columns: {}).",
                t.name,
                names_of(t.columns.iter().map(|c| c.name.as_str()))
            ),
        )
    })
}

/// `(c0, header row, c1, last row)` of a table's range.
fn table_area(t: &TableDecl) -> (u32, u32, u32, u32) {
    area(&t.range).unwrap_or((0, 1, 0, 1))
}

fn set_rows(t: &mut TableDecl, last: u32) {
    let (c0, r0, c1, _) = table_area(t);
    t.range = format!("{}{r0}:{}{}", letters(c0), letters(c1), last.max(r0));
}

fn letters(col: u32) -> String {
    let mut s = Vec::new();
    let mut n = col + 1;
    while n > 0 {
        s.push(b'A' + ((n - 1) % 26) as u8);
        n = (n - 1) / 26;
    }
    s.reverse();
    String::from_utf8(s).unwrap()
}

/// Check a value written into `col` (§8: typed by the column, never guessed).
pub fn value_problem(col_name: &str, ty: &str, v: &Value) -> Option<String> {
    match (ty, v) {
        (_, Value::Null) => None,
        ("number", Value::Number(_)) => None,
        ("number", Value::Text(s)) => Some(format!(
            "column \"{col_name}\" is a number column; {s:?} is text. Write a JSON number ({}), or change the column's type with set_type.",
            s.replace(',', "").trim()
        )),
        ("text", Value::Text(_)) => None,
        ("text", Value::Number(n)) => Some(format!(
            "column \"{col_name}\" is a text column; write {n} as a string (\"{n}\"). IDs, codes and phone numbers are text, with their leading zeros."
        )),
        ("date", Value::Text(s)) if date_of(s).is_some() => None,
        ("date", _) => Some(format!("column \"{col_name}\" is a date column; write a date as \"YYYY-MM-DD\" (or \"YYYY-MM\" for the first of the month), not {v}.")),
        ("mixed", _) => None,
        (_, Value::Bool(_)) => Some(format!("column \"{col_name}\" is a {ty} column; true and false go in a mixed column.")),
        _ => Some(format!("column \"{col_name}\" is a {ty} column; {v} does not fit it.")),
    }
}

/// `(year, month, day)` of `YYYY-MM-DD` or `YYYY-MM`.
pub fn date_of(s: &str) -> Option<(i32, u32, u32)> {
    let p: Vec<&str> = s.trim().split('-').collect();
    let num = |x: &str, n: usize| {
        (x.len() == n && x.bytes().all(|b| b.is_ascii_digit())).then(|| x.parse::<u32>().ok()).flatten()
    };
    let (y, m, d) = match p.as_slice() {
        [y, m] => (num(y, 4)?, num(m, 2)?, 1),
        [y, m, d] => (num(y, 4)?, num(m, 2)?, num(d, 2)?),
        _ => return None,
    };
    let dim = [
        31,
        if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    ((1900..=9999).contains(&y) && (1..=12).contains(&m) && d >= 1 && d <= dim[m as usize - 1])
        .then_some((y as i32, m, d))
}

fn check_rows(cx: &Ctx, t: &TableDecl, rows: &[Row]) -> Result<(), OpError> {
    for (k, r) in rows.iter().enumerate() {
        for (c, v) in r {
            let ci = column_index(cx, "rows", t, c)
                .map_err(|e| OpError { message: format!("row {}: {}", k + 1, e.message), ..e })?;
            let col = &t.columns[ci];
            if !col.formula.is_empty() {
                if *v == Value::Null {
                    continue;
                }
                return Err(cx.err(
                    "rows",
                    format!(
                        "row {}: {}[{}] is a formula column; its cells are computed. Leave it out of new rows.",
                        k + 1,
                        t.name,
                        c
                    ),
                ));
            }
            if let Some(m) = value_problem(c, &col.ty, v) {
                return Err(cx.err("rows", format!("row {}: {m}", k + 1)));
            }
        }
    }
    Ok(())
}

fn check_format(cx: &Ctx, field: &str, formats: &Formats, ty: &str, fmt: &str) -> Result<(), OpError> {
    if !formats.allowed(fmt) {
        return Err(
            cx.err(field, format!("\"{fmt}\" is not a number format of this workbook. Formats: {}.", formats.list()))
        );
    }
    if !type_fits_format(ty, fmt) {
        let want = match ty {
            "text" => "a text column's format is @".to_string(),
            "date" => "a date column takes a date format (yyyy-mm-dd, yyyy-mm, …)".to_string(),
            "number" => "a number column takes a number format (#,##0, 0.0%, 00000, …)".to_string(),
            _ => String::new(),
        };
        return Err(cx.err(field, format!("\"{fmt}\" does not fit a {ty} column: {want}.")));
    }
    Ok(())
}

fn default_format(ty: &str) -> Option<&'static str> {
    match ty {
        "text" => Some("@"),
        "mixed" => Some("General"),
        _ => None,
    }
}

fn spec_to_decl(cx: &Ctx, field: &str, formats: &Formats, c: &ColumnSpec) -> Result<ColumnDecl, OpError> {
    if !["text", "number", "date"].contains(&c.ty.as_str()) {
        return Err(cx.err(
            field,
            format!("column \"{}\": the type is \"text\", \"number\" or \"date\", not \"{}\".", c.name, c.ty),
        ));
    }
    let format = match (&c.format, default_format(&c.ty)) {
        (Some(f), _) => f.clone(),
        (None, Some(d)) => d.to_string(),
        (None, None) => return Err(cx.err(field, format!("column \"{}\": a {} column needs a format.", c.name, c.ty))),
    };
    check_format(cx, field, formats, &c.ty, &format)?;
    Ok(ColumnDecl { name: c.name.clone(), ty: c.ty.clone(), format, formula: c.formula.clone().unwrap_or_default() })
}

/// A model-written formula for a column of table `own` (with `own_cols`), against the structure.
fn check_formula(
    cx: &Ctx,
    field: &str,
    st: &Spreadsheet,
    own: &str,
    own_cols: &[ColumnDecl],
    f: &str,
) -> Result<(), OpError> {
    if let Some(p) = formula::model_formula_problem(f) {
        let m = match p {
            FormulaProblem::NotAFormula => format!("a formula is text starting with \"=\", not {f:?}."),
            p => format!("formula {f} is refused: {p}."),
        };
        return Err(cx.err(field, m));
    }
    for r in formula::structured_refs(f) {
        let (tname, cols): (String, Vec<&str>) = match &r.table {
            None => (own.to_string(), own_cols.iter().map(|c| c.name.as_str()).collect()),
            Some(t) if t.eq_ignore_ascii_case(own) => {
                (own.to_string(), own_cols.iter().map(|c| c.name.as_str()).collect())
            }
            Some(t) => match st.table(t) {
                Some((_, td)) => (td.name.clone(), td.columns.iter().map(|c| c.name.as_str()).collect()),
                None => {
                    return Err(cx.err(
                        field,
                        format!(
                            "formula {f}: there is no table \"{t}\". Tables: {}.",
                            names_of(st.tables().map(|(_, t)| t.name.as_str()))
                        ),
                    ))
                }
            },
        };
        if r.this_row() && r.table.as_deref().is_some_and(|t| !t.eq_ignore_ascii_case(own)) {
            return Err(cx.err(
                field,
                format!(
                    "formula {f}: [@…] is this row of the formula's own table; another table is read as Table[Column]."
                ),
            ));
        }
        if let Some((a, b)) = &r.columns {
            for c in [a, b] {
                if !cols.contains(&c.as_str()) {
                    return Err(cx.err(
                        field,
                        format!(
                            "formula {f}: table {tname} has no column \"{c}\" (columns: {}).",
                            names_of(cols.iter().copied())
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn check_one(cx: &Ctx, op: &RangeOp, st: &mut Spreadsheet, formats: &Formats) -> Result<(), OpError> {
    match op {
        RangeOp::Set { range, values } => {
            let form = "is Sheet!Cell or Sheet!First:Last, such as \"매출!D70\" or \"매출!D70:E71\"";
            let (sheet, cells) =
                split_range(range).ok_or_else(|| cx.err("range", format!("{form}, not {range:?}.")))?;
            let Some(sd) = st.sheet(&sheet) else {
                return Err(cx.err(
                    "range",
                    format!(
                        "there is no sheet \"{sheet}\". Sheets: {}.",
                        names_of(st.sheets.iter().map(|s| s.name.as_str()))
                    ),
                ));
            };
            let (c0, r0, c1, r1) = area(&cells).ok_or_else(|| cx.err("range", format!("{form}, not {range:?}.")))?;
            if c1 < c0 || r1 < r0 {
                return Err(cx.err("range", format!("{range} runs backwards; the first cell is the top-left one.")));
            }
            let (h, w) = ((r1 - r0 + 1) as usize, (c1 - c0 + 1) as usize);
            if values.len() != h || values.iter().any(|r| r.len() != w) {
                return Err(
                    cx.err("values", format!("is a list of {h} row(s) of {w} value(s) each, the shape of {range}."))
                );
            }
            for (i, row) in values.iter().enumerate() {
                for (j, v) in row.iter().enumerate() {
                    let (c, r) = (c0 + j as u32, r0 + i as u32);
                    let at = format!("{sheet}!{}{r}", letters(c));
                    if let Some(t) = table_at(sd, c, r) {
                        let (tc0, tr0, _, _) = table_area(t);
                        let col = &t.columns[(c - tc0) as usize];
                        if r == tr0 {
                            return Err(cx.err(
                                "range",
                                format!(
                                    "{at} is the header of table {}; header cells are column names, not data.",
                                    t.name
                                ),
                            ));
                        }
                        if !col.formula.is_empty() {
                            return Err(cx.err("range", format!("{at} is in {}[{}], a formula column; its cells are computed from {} and cannot be set.", t.name, col.name, col.formula)));
                        }
                        if let Some(m) = value_problem(&col.name, &col.ty, v) {
                            return Err(cx.err("values", format!("{at}: {m}")));
                        }
                    }
                }
            }
        }
        RangeOp::AppendRows { table, rows } => {
            let at = find_table(cx, st, table)?;
            let t = table_mut(st, at);
            check_rows(cx, t, rows)?;
            let last = table_area(t).3;
            set_rows(t, last + rows.len() as u32);
        }
        RangeOp::InsertRows { table, before, rows } => {
            let at = find_table(cx, st, table)?;
            let t = table_mut(st, at);
            let (_, r0, _, r1) = table_area(t);
            if *before <= r0 || *before > r1 + 1 {
                return Err(cx.err(
                    "before",
                    format!(
                        "{before} is not a data row of {} (rows {}-{}, or {} to add after the last).",
                        t.name,
                        r0 + 1,
                        r1,
                        r1 + 1
                    ),
                ));
            }
            check_rows(cx, t, rows)?;
            set_rows(t, r1 + rows.len() as u32);
        }
        RangeOp::DeleteRows { table, first, last } => {
            let at = find_table(cx, st, table)?;
            let t = table_mut(st, at);
            let (_, r0, _, r1) = table_area(t);
            if *first <= r0 || *last > r1 {
                return Err(cx.err(
                    "rows",
                    format!("rows {first}-{last} are not data rows of {} (rows {}-{}).", t.name, r0 + 1, r1),
                ));
            }
            set_rows(t, r1 - (last - first + 1));
        }
        RangeOp::FillFormula { table, column, formula } => {
            let at = find_table(cx, st, table)?;
            let snapshot = st.clone();
            let t = table_mut(st, at);
            let ci = column_index(cx, "column", t, column)?;
            check_formula(cx, "formula", &snapshot, &t.name.clone(), &t.columns.clone(), formula)?;
            let refs_self = formula::structured_refs(formula).iter().any(|r| {
                r.table.as_deref().is_none_or(|x| x.eq_ignore_ascii_case(&t.name))
                    && r.columns.as_ref().is_some_and(|(a, b)| a == column || b == column)
                    && r.this_row()
            });
            if refs_self {
                return Err(cx.err("formula", format!("the formula of {}[{column}] refers to its own cell.", t.name)));
            }
            t.columns[ci].formula = formula.clone();
        }
        RangeOp::SetType { table, column, ty, format } => {
            let at = find_table(cx, st, table)?;
            let t = table_mut(st, at);
            let ci = column_index(cx, "column", t, column)?;
            if !["text", "number", "date"].contains(&ty.as_str()) {
                return Err(cx.err("type", format!("is \"text\", \"number\" or \"date\", not \"{ty}\".")));
            }
            let col = &t.columns[ci];
            let fmt = match (format, default_format(ty)) {
                (Some(f), _) => f.clone(),
                (None, Some(d)) => d.to_string(),
                (None, None) if col.ty == *ty => col.format.clone(),
                (None, None) => return Err(cx.err("format", format!("a {ty} column needs a format."))),
            };
            check_format(cx, "format", formats, ty, &fmt)?;
            if (col.ty == "number" && ty == "date") || (col.ty == "date" && ty == "number") {
                return Err(cx.err(
                    "type",
                    format!("a {} column cannot become a {ty} column directly; make it text first.", col.ty),
                ));
            }
            let c = &mut t.columns[ci];
            c.ty = ty.clone();
            c.format = fmt;
        }
        RangeOp::AddColumn { table, column } => {
            let at = find_table(cx, st, table)?;
            let snapshot = st.clone();
            let t = table_mut(st, at);
            if t.columns.iter().any(|c| c.name.eq_ignore_ascii_case(&column.name)) {
                return Err(cx.err("column", format!("table {} already has a column \"{}\".", t.name, column.name)));
            }
            let decl = spec_to_decl(cx, "column", formats, column)?;
            let mut cols = t.columns.clone();
            cols.push(decl.clone());
            if !decl.formula.is_empty() {
                check_formula(cx, "column", &snapshot, &t.name.clone(), &cols, &decl.formula)?;
            }
            t.columns.push(decl);
            let (c0, r0, c1, r1) = table_area(t);
            t.range = format!("{}{r0}:{}{r1}", letters(c0), letters(c1 + 1));
        }
        RangeOp::Sort { table, keys } => {
            let at = find_table(cx, st, table)?;
            let t = table_mut(st, at);
            for k in keys {
                column_index(cx, "keys", t, &k.column)?;
            }
        }
        RangeOp::AddTable { sheet, name, anchor, columns, rows } => {
            let Some(si) = st.sheets.iter().position(|s| s.name.eq_ignore_ascii_case(sheet)) else {
                return Err(cx.err(
                    "sheet",
                    format!(
                        "there is no sheet \"{sheet}\". Sheets: {}.",
                        names_of(st.sheets.iter().map(|s| s.name.as_str()))
                    ),
                ));
            };
            if let Some(m) = table_name_problem(name) {
                return Err(cx.err("name", format!("\"{name}\": {m}.")));
            }
            if st.table(name).is_some() {
                return Err(cx.err(
                    "name",
                    format!("there is already a table \"{name}\"; table names are unique in the workbook."),
                ));
            }
            let (c0, r0) = cell(&anchor.replace('$', "").to_ascii_uppercase()).ok_or_else(|| {
                cx.err("anchor", format!("is the cell of the table's top-left header, such as \"H1\", not {anchor:?}."))
            })?;
            let mut decls: Vec<ColumnDecl> = vec![];
            for c in columns {
                if decls.iter().any(|d| d.name.eq_ignore_ascii_case(&c.name)) {
                    return Err(cx.err("columns", format!("two columns are named \"{}\".", c.name)));
                }
                decls.push(spec_to_decl(cx, "columns", formats, c)?);
            }
            // A table has at least one data row (a blank one when none is given).
            let c1 = c0 + decls.len() as u32 - 1;
            let r1 = (r0 + rows.len() as u32).max(r0 + 1);
            let decl = TableDecl {
                name: name.clone(),
                range: format!("{}{r0}:{}{r1}", letters(c0), letters(c1)),
                columns: decls,
            };
            for c in &decl.columns {
                if !c.formula.is_empty() {
                    let snapshot = st.clone();
                    check_formula(cx, "columns", &snapshot, name, &decl.columns, &c.formula)?;
                }
            }
            check_rows(cx, &decl, rows)?;
            let mine = (c0, r0, c1, r1);
            for (_, other) in st.tables().filter(|(s, _)| s.name.eq_ignore_ascii_case(sheet)) {
                let o = table_area(other);
                if mine.0 <= o.2 && o.0 <= mine.2 && mine.1 <= o.3 && o.1 <= mine.3 {
                    return Err(cx.err(
                        "anchor",
                        format!(
                            "a table at {anchor} would overlap table {} ({}); tables on a sheet never overlap.",
                            other.name, other.range
                        ),
                    ));
                }
            }
            st.sheets[si].items.push(SheetItem::Table(decl));
        }
        RangeOp::AddSheet { name } => {
            if let Some(m) = sheet_name_problem(name) {
                return Err(cx.err("name", format!("\"{name}\": {m}.")));
            }
            if st.sheet(name).is_some() {
                return Err(cx.err("name", format!("there is already a sheet \"{name}\".")));
            }
            st.sheets.push(SheetDecl { name: name.clone(), range: None, items: vec![] });
        }
    }
    Ok(())
}

/// The table of sheet `s` holding cell (`c`, `r`), if any.
fn table_at(s: &SheetDecl, c: u32, r: u32) -> Option<&TableDecl> {
    s.items.iter().find_map(|i| match i {
        SheetItem::Table(t) => {
            let (c0, r0, c1, r1) = table_area(t);
            ((c0..=c1).contains(&c) && (r0..=r1).contains(&r)).then_some(t)
        }
        _ => None,
    })
}

/// Whether a format code is a date format (for callers typing values).
pub fn is_date_format(code: &str) -> bool {
    format_kind(code) == FormatKind::Date
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::names::Names;
    use crate::sheet::parse_spreadsheet;

    const TEXT: &str = "---\ntype: spreadsheet\nformat: xlsx\nschema: 1\n---\n<sheet name=\"매출\">\n<table name=\"Sales\" range=\"A1:D10\">\n| column | type | format | formula |\n|---|---|---|---|\n| 월 | date | yyyy-mm |  |\n| 지점 | text | @ |  |\n| 매출 | number | #,##0 |  |\n| 이익 | number | #,##0 | =[@매출]*0.1 |\n</table>\n</sheet>\n";

    fn check(json: &str) -> Result<(), String> {
        let st = parse_spreadsheet(TEXT, &Names::default()).unwrap();
        let ops = parse_ops(json).map_err(|e| render_errors(&e))?;
        check_ops(&ops, &st, &Formats::default()).map_err(|e| render_errors(&e))
    }

    #[test]
    fn the_round4_example_passes() {
        check(r##"[{"op": "set", "range": "매출!C3", "values": [[15300000]]},
                  {"op": "append_rows", "table": "Sales", "rows": [{"월": "2026-03", "지점": "강남", "매출": 17200000}]}]"##)
        .unwrap();
        check(r##""[{\"op\": \"add_sheet\", \"name\": \"요약\"}]""##).unwrap();
        check("```json\n[{\"op\": \"sort\", \"table\": \"Sales\", \"keys\": [{\"column\": \"매출\", \"order\": \"desc\"}]}]\n```").unwrap();
    }

    #[test]
    fn errors_name_the_operation_field_and_allowed_names() {
        let e = check(r##"[{"op": "set", "range": "매출!C3", "values": [[1]]}, {"op": "apend_rows"}]"##).unwrap_err();
        assert!(
            e.starts_with("operation 2: \"apend_rows\" is not an operation; \"op\" is one of set, append_rows"),
            "{e}"
        );
        let e = check(r##"[{"op": "append_rows", "table": "Sale", "rows": []}]"##).unwrap_err();
        assert_eq!(e, "operation 1 (append_rows), \"table\": there is no table \"Sale\". Tables: \"Sales\".");
        let e = check(r##"[{"op": "append_rows", "table": "Sales", "rows": [{"매출": "1,204"}]}]"##).unwrap_err();
        assert!(
            e.contains("row 1: column \"매출\" is a number column; \"1,204\" is text. Write a JSON number (1204)"),
            "{e}"
        );
        let e = check(r##"[{"op": "append_rows", "table": "Sales", "rows": [{"이익": 5}]}]"##).unwrap_err();
        assert!(e.contains("formula column"), "{e}");
        let e = check(r##"[{"op": "set", "range": "매출!D3", "values": [[1]]}]"##).unwrap_err();
        assert!(e.contains("formula column"), "{e}");
        let e = check(r##"[{"op": "set", "range": "매출!C1", "values": [[1]]}]"##).unwrap_err();
        assert!(e.contains("header of table Sales"), "{e}");
        let e = check(r##"[{"op": "set", "range": "매출!C3:C4", "values": [[1]]}]"##).unwrap_err();
        assert!(e.contains("2 row(s) of 1 value(s)"), "{e}");
        let e = check(r##"[{"op": "fill_formula", "table": "Sales", "column": "이익", "formula": "=C2-D2"}]"##)
            .unwrap_err();
        assert!(e.contains("A1 reference C2"), "{e}");
        let e = check(r##"[{"op": "add_column", "table": "Sales", "column": {"name": "환율", "type": "number", "format": "0.0", "formula": "=WEBSERVICE(\"https://x\")"}}]"##).unwrap_err();
        assert!(e.contains("fetches data"), "{e}");
        let e =
            check(r##"[{"op": "fill_formula", "table": "Sales", "column": "이익", "formula": "=[@매출]-[@원가]"}]"##)
                .unwrap_err();
        assert!(e.contains("no column \"원가\""), "{e}");
        let e = check(
            r##"[{"op": "set_type", "table": "Sales", "column": "지점", "type": "number", "format": "yyyy-mm"}]"##,
        )
        .unwrap_err();
        assert!(e.contains("does not fit a number column"), "{e}");
        let e = check(r##"[{"op": "delete_rows", "table": "Sales", "rows": "1:2"}]"##).unwrap_err();
        assert!(e.contains("rows 2-10"), "{e}");
        let e = check(r##"[{"op": "add_table", "sheet": "매출", "name": "Returns", "anchor": "C5", "columns": [{"name": "a", "type": "text"}]}]"##).unwrap_err();
        assert!(e.contains("overlap"), "{e}");
        let rows: Vec<String> = (0..51).map(|_| "{\"지점\": \"x\"}".to_string()).collect();
        let e = check(&format!("[{{\"op\": \"append_rows\", \"table\": \"Sales\", \"rows\": [{}]}}]", rows.join(",")))
            .unwrap_err();
        assert!(e.contains("no answer adds more than 50 rows"), "{e}");
        let e = check(r##"[{"op": "delete_rows", "table": "Sales", "row": "3"}]"##).unwrap_err();
        assert_eq!(e, "operation 1 (delete_rows): it takes table, rows; missing rows.");
    }

    #[test]
    fn later_operations_see_earlier_ones() {
        check(r##"[{"op": "add_sheet", "name": "요약"},
                  {"op": "add_table", "sheet": "요약", "name": "Hiring", "anchor": "B2", "columns": [{"name": "인원", "type": "number", "format": "0"}, {"name": "합", "type": "number", "format": "#,##0", "formula": "=[@인원]*SUM(Sales[매출])"}], "rows": [{"인원": 3}]},
                  {"op": "append_rows", "table": "Hiring", "rows": [{"인원": 4}]},
                  {"op": "add_column", "table": "Sales", "column": {"name": "비고", "type": "text"}},
                  {"op": "set", "range": "매출!E3", "values": [["메모"]]}]"##)
        .unwrap();
    }
}
