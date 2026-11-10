//! The Spreadsheet grammar (DESIGN.md §5.4): the workbook's structure as text
//! — `<sheet>` blocks holding `<table>` blocks (one pipe-table line per
//! column: name, type, number format, formula), `<merge range/>` lines,
//! `<hyperlink/>` lines, `<note/>` lines, `<validation/>` lines,
//! `<conditional-format/>` lines, drawing `<shape>` / `<group>` lines,
//! `<chart/>` lines and placeholders — and the row window through which
//! cell data is read (a pipe table whose first column is the read-only
//! sheet row number, values as displayed; §6 round 4, read view A). Cell
//! data is written by range operations (`ops.rs`), never as text.

use crate::ast::{FrontMatter, Keep};
use crate::cellfmt::{self, CellFormat, FormatLine};
use crate::diag::{quoted, Diagnostic};
use crate::names::Names;
use crate::parse::{chars_of, decode_attr, parse_tag, Parser};
use crate::serialize::{attr, front_lines};
use crate::vocab::Color;

/// Column types (§5.4, §8: typed by the source, never guessed). `mixed` is
/// a column whose cells do not share one type; each cell keeps its own.
pub const COLUMN_TYPES: &[&str] = &["text", "number", "date", "mixed"];

/// Chart types a `<chart/>` line may name.
pub const CHART_TYPES: &[&str] = &[
    "bar", "column", "line", "pie", "area", "scatter", "doughnut", "radar",
];

/// At most this many `<merge range/>` lines on one sheet. A sheet with more
/// (tens of thousands of merged ranges) keeps them as a placeholder: the
/// structure text does not grow with the grid (§2 rule 11).
pub const MAX_MERGE_LINES: usize = 1024;

/// At most this many `<hyperlink/>` lines on one sheet. A sheet with more
/// keeps them as a placeholder (§2 rule 11).
pub const MAX_HYPERLINK_LINES: usize = 1024;

/// At most this many `<note/>` lines on one sheet. A sheet with more keeps
/// them as a placeholder (§2 rule 11).
pub const MAX_NOTE_LINES: usize = 1024;

/// At most this many `<validation/>` lines on one sheet. A sheet with more
/// keeps them as a placeholder (§2 rule 11).
pub const MAX_VALIDATION_LINES: usize = 1024;

/// At most this many `<conditional-format/>` lines on one sheet. A sheet
/// with more keeps them as a placeholder (§2 rule 11).
pub const MAX_CONDITIONAL_FORMAT_LINES: usize = 1024;

/// At most this many `<shape>` lines on one sheet (grouped shapes count).
/// A sheet with more keeps them as a placeholder (§2 rule 11).
pub const MAX_SHAPE_LINES: usize = 1024;

/// Data-validation types a `<validation/>` line may name (SpreadsheetML).
pub const VALIDATION_TYPES: &[&str] = &[
    "whole",
    "decimal",
    "list",
    "date",
    "time",
    "textLength",
    "custom",
];

/// Comparison operators a validation or `cellIs` rule may name.
pub const VALIDATION_OPERATORS: &[&str] = &[
    "between",
    "notBetween",
    "equal",
    "notEqual",
    "greaterThan",
    "lessThan",
    "greaterThanOrEqual",
    "lessThanOrEqual",
];

/// `errorStyle` values Excel writes.
pub const VALIDATION_ERROR_STYLES: &[&str] = &["stop", "warning", "information"];

/// Conditional-format rule types the text models. Other types (colorScale,
/// iconSet, …) stay a placeholder.
pub const CONDITIONAL_FORMAT_TYPES: &[&str] = &["cellIs", "expression", "dataBar"];

/// Data-bar `min`/`max` types (`min="num:0"`, `max="max"`). Round 7 A.
pub const CFVO_TYPES: &[&str] = &["min", "max", "num", "percent", "percentile", "formula"];

/// Excel's last row and column (A1's `XFD1048576`).
const MAX_ROW: u32 = 1_048_576;
const MAX_COL: u32 = 16_384;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spreadsheet {
    pub front: FrontMatter,
    /// The Normal style's formatting (`<format default …/>`), when the
    /// workbook shows formatting (read-only: §5.4, the `format` operation).
    pub default_format: Option<CellFormat>,
    pub sheets: Vec<SheetDecl>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetDecl {
    pub name: String,
    /// The used range, as the file has it (read-only; `None` for an empty sheet).
    pub range: Option<String>,
    /// The sheet's formatted cells, one line per rectangle (read-only).
    pub formats: Vec<FormatLine>,
    pub items: Vec<SheetItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SheetItem {
    Table(TableDecl),
    Merge(MergeDecl),
    Hyperlink(HyperlinkDecl),
    Note(NoteDecl),
    Validation(ValidationDecl),
    /// Boxed: `CondFormatDecl` holds a `CellFormat`, which would otherwise
    /// dominate `SheetItem`'s size (clippy `large_enum_variant`).
    ConditionalFormat(Box<CondFormatDecl>),
    /// A drawing shape (`<shape name="…" anchor="B2:D5">text</shape>`).
    Shape(ShapeDecl),
    /// A group of drawing shapes (`<group name="…">` … `</group>`).
    Group(GroupDecl),
    Chart(ChartDecl),
    Keep(Keep),
}

/// One drawing shape on a sheet (`<shape name="Tip" anchor="B25:D27">text</shape>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapeDecl {
    pub name: String,
    /// Canonical A1 cell or range of Excel's two-cell (or one-cell) anchor.
    pub anchor: String,
    /// Plain text of the shape; a newline is `&#10;` in the body. Empty
    /// when the shape has none (`<shape …/>`).
    pub text: String,
}

/// One group of drawing shapes (`<group name="신호등">` … `</group>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupDecl {
    pub name: String,
    pub items: Vec<DrawingItem>,
}

/// A shape or nested group inside a [`GroupDecl`] (or collected from a
/// sheet's drawing lines).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DrawingItem {
    Shape(ShapeDecl),
    Group(GroupDecl),
}

/// One merged range on a sheet (`<merge range="A1:F1"/>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeDecl {
    /// Canonical A1 range (`A1:F1`), two or more cells.
    pub range: String,
}

/// One cell hyperlink (`<hyperlink range="A1" target="https://example.com"/>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyperlinkDecl {
    /// Canonical A1 cell or range (`A1`, `A3:A5`).
    pub range: String,
    /// External URL (`TargetMode="External"` relationship), when there is one.
    pub target: Option<String>,
    /// Location inside the target (a sheet cell, a defined name, a bookmark).
    pub location: Option<String>,
    /// Optional display string stored on the hyperlink, not the cell value.
    pub display: Option<String>,
    /// Optional tooltip.
    pub tooltip: Option<String>,
}

/// One cell note (`<note range="B2" author="John" text="Some text"/>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteDecl {
    /// Canonical A1 cell or range the note is anchored to.
    pub range: String,
    pub author: String,
    /// Plain text of the balloon; a newline is `&#10;` in the attribute.
    pub text: String,
}

/// One data validation (`<validation range="C2" type="list" formula1="&quot;A,B,C&quot;"/>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationDecl {
    /// Canonical A1 range or space-separated ranges (`C2`, `B4:B48`).
    pub range: String,
    /// SpreadsheetML type (`list`, `whole`, `decimal`, …).
    pub ty: String,
    /// Omitted when the file uses the default (`between`).
    pub operator: Option<String>,
    pub formula1: Option<String>,
    pub formula2: Option<String>,
    /// Written `allowBlank="true"` when the file allows blanks
    /// (SpreadsheetML default is false).
    pub allow_blank: bool,
    /// Text default is on (omit). `Some(false)` when the alert is off.
    /// SpreadsheetML defaults to false when the XML attribute is absent.
    pub show_input_message: Option<bool>,
    /// Text default is on (omit). `Some(false)` when the alert is off.
    /// SpreadsheetML defaults to false when the XML attribute is absent.
    pub show_error_message: Option<bool>,
    pub prompt_title: Option<String>,
    pub prompt: Option<String>,
    pub error_title: Option<String>,
    pub error: Option<String>,
    /// Omitted when the file uses the default (`stop`).
    pub error_style: Option<String>,
}

/// A data-bar threshold as one token (`min`, `num:0`, `formula:=A1`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cfvo {
    pub ty: String,
    /// Present when `ty` is not `min` or `max`.
    pub val: Option<String>,
}

impl Cfvo {
    /// Parse `min`, `max`, `num:0`, `percent:50`, `percentile:90` or
    /// `formula:=A1`. A formula without `=` is stored with one.
    pub fn parse(s: &str) -> Result<Cfvo, String> {
        let form = "min, max, num:0, percent:50, percentile:90 or formula:=A1";
        let (ty, val) = match s.split_once(':') {
            Some((ty, rest)) => (ty, Some(rest)),
            None => (s, None),
        };
        if !CFVO_TYPES.contains(&ty) {
            return Err(format!("{s:?} is not a data-bar min or max; write {form}."));
        }
        match (ty, val) {
            ("min" | "max", Some(_)) => Err(format!("{ty} takes no value; write {ty}, not {s:?}.")),
            ("min" | "max", None) => Ok(Cfvo {
                ty: ty.to_string(),
                val: None,
            }),
            (_, None) | (_, Some("")) => Err(format!(
                "{ty} needs a value: {ty}:0, {ty}:name or {ty}:=A1."
            )),
            ("percent" | "percentile", Some(v)) => {
                let n: f64 = v
                    .parse()
                    .map_err(|_| format!("{ty} \"{v}\" is not a number 0–100."))?;
                if !(0.0..=100.0).contains(&n) || !n.is_finite() {
                    return Err(format!("{ty} \"{v}\" is not a number 0–100."));
                }
                Ok(Cfvo {
                    ty: ty.to_string(),
                    val: Some(v.to_string()),
                })
            }
            ("formula", Some(v)) => {
                let v = if v.starts_with('=') {
                    v.to_string()
                } else {
                    format!("={v}")
                };
                Ok(Cfvo {
                    ty: ty.to_string(),
                    val: Some(v),
                })
            }
            ("num", Some(v)) => Ok(Cfvo {
                ty: ty.to_string(),
                val: Some(v.to_string()),
            }),
            _ => Err(format!("{s:?} is not a data-bar min or max; write {form}.")),
        }
    }

    /// Canonical token (`num:0`, `max`, `formula:=A1`).
    pub fn write(&self) -> String {
        match &self.val {
            None => self.ty.clone(),
            Some(v) => format!("{}:{v}", self.ty),
        }
    }
}

/// One conditional-format rule (`<conditional-format range="A1" type="cellIs"
/// operator="greaterThan" formula="5" dxf="fill=#FFC7CE"/>`, or
/// `type="dataBar" min="num:0" max="max" fill="#638EC6"`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CondFormatDecl {
    /// Canonical A1 range or space-separated ranges.
    pub range: String,
    /// `cellIs`, `expression` or `dataBar`.
    pub ty: String,
    /// Omitted when the file uses the default (`between`), and for
    /// `expression` and `dataBar` rules.
    pub operator: Option<String>,
    pub formula: Option<String>,
    pub formula2: Option<String>,
    /// Differential formatting; empty when the rule has none.
    pub dxf: CellFormat,
    /// Written `stopIfTrue="true"` when the file stops at this rule.
    pub stop_if_true: bool,
    /// Data bar: `min="num:0"`. Required for `dataBar`, absent otherwise.
    pub min: Option<Cfvo>,
    pub max: Option<Cfvo>,
    /// Data bar fill (`#638EC6`, `accent1`).
    pub fill: Option<Color>,
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
        self.tables()
            .find(|(_, t)| t.name.eq_ignore_ascii_case(name))
    }

    pub fn sheet(&self, name: &str) -> Option<&SheetDecl> {
        self.sheets
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
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
#[non_exhaustive]
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
        return Some(format!(
            "a sheet name cannot hold {c:?} (nor any of [ ] : * ? / \\)"
        ));
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
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '\\'))
    {
        return Some(form.into());
    }
    if name.chars().count() > 255 {
        return Some("a table name has at most 255 characters".into());
    }
    let upper = name.to_ascii_uppercase();
    let cell_like = hanji_cell_like(&upper) || upper == "R" || upper == "C";
    if cell_like {
        return Some(format!(
            "{name} reads as a cell address; {form}, and not a cell address such as A1 or R1C1"
        ));
    }
    None
}

fn hanji_cell_like(u: &str) -> bool {
    let k = u
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(u.len());
    let a1 = k > 0 && k <= 3 && k < u.len() && u[k..].bytes().all(|b| b.is_ascii_digit());
    let r1c1 = u.starts_with('R') && {
        let rest = u[1..].trim_start_matches(|c: char| c.is_ascii_digit());
        rest.starts_with('C') && rest[1..].bytes().all(|b| b.is_ascii_digit())
    };
    a1 || r1c1
}

const SHEET_FORM: &str = "<sheet name=\"…\"> … </sheet>";
const TABLE_FORM: &str = "<table name=\"…\" range=\"A1:F20\">, the lines | column | type | format | formula | and |---|---|---|---|, one line | name | type | format | formula | per column, and </table>";
const MERGE_FORM: &str = "<merge range=\"A1:B2\"/>";
const HYPERLINK_FORM: &str =
    "<hyperlink range=\"A1\" target=\"https://example.com\"/> or <hyperlink range=\"B2\" location=\"Sheet2!A1\"/>";
const NOTE_FORM: &str = "<note range=\"B2\" author=\"…\" text=\"…\"/>";
const VALIDATION_FORM: &str =
    "<validation range=\"C2\" type=\"list\" formula1=\"&quot;A,B,C&quot;\"/> or <validation range=\"B2:B10\" type=\"whole\" operator=\"between\" formula1=\"1\" formula2=\"100\" allowBlank=\"true\"/>";
const CONDITIONAL_FORMAT_FORM: &str =
    "<conditional-format range=\"A1:A10\" type=\"cellIs\" operator=\"greaterThan\" formula=\"5\" dxf=\"fill=#FFC7CE\"/> or <conditional-format range=\"E4\" type=\"dataBar\" min=\"num:0\" max=\"max\" fill=\"#638EC6\"/>";
const CHART_FORM: &str = "<chart type=\"bar\" data=\"Table[Column],Table[Column]\" title=\"…\"/>";
const SHAPE_FORM: &str =
    "<shape name=\"…\" anchor=\"B2:D5\">text</shape> or <shape name=\"…\" anchor=\"B2:D5\"/>";
const GROUP_FORM: &str = "<group name=\"…\">, one <shape> line per member, and </group>";
const KEEP_FORM: &str = "<keep id=\"…\" kind=\"…\" summary=\"…\"/>";

/// Parse the structure text of a Spreadsheet, checking placeholders
/// against `names.keeps` (a `None` list skips that check).
pub fn parse_spreadsheet(text: &str, names: &Names) -> Result<Spreadsheet, Vec<Diagnostic>> {
    let mut p = Parser::new(text, names);
    let (front, first) = p.front_matter("spreadsheet");
    let Some(front) = front.filter(|_| p.errors.is_empty()) else {
        return Err(p.errors);
    };
    let mut sheets: Vec<SheetDecl> = vec![];
    let mut keeps: Vec<(usize, usize, Keep)> = vec![];
    let n = p.lines.len();
    let mut i = first;
    let mut open: Option<(usize, SheetDecl)> = None;
    let mut default_format: Option<CellFormat> = None;
    let mut n_shapes: usize = 0;
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
                None => p.err(
                    i,
                    col,
                    format!("</sheet> closes no sheet; a sheet is {SHEET_FORM}."),
                ),
            }
            n_shapes = 0;
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
            n_shapes = 0;
            i += 1;
            continue;
        }
        if t.starts_with("<format") {
            match cellfmt::parse_line(t) {
                Ok(cellfmt::Parsed::Default(f)) if open.is_none() && sheets.is_empty() && default_format.is_none() => {
                    default_format = Some(f)
                }
                Ok(cellfmt::Parsed::Default(_)) => p.err(
                    i,
                    col,
                    "the line <format default …/> comes once, after the front matter and before the first sheet.",
                ),
                Ok(cellfmt::Parsed::Range(l)) => match open.as_mut() {
                    Some((_, sheet)) if is_range(&l.range) => sheet.formats.push(l),
                    Some(_) => p.err(i, col, format!("range=\"{}\" is not an A1 range such as B4:C7.", l.range)),
                    None => p.err(i, col, format!("a <format range=…/> line is inside a sheet: {SHEET_FORM}.")),
                },
                Err((c, m)) => p.err(i, col + c - 1, format!("{m}.")),
            }
            i += 1;
            continue;
        }
        let Some((_, sheet)) = open.as_mut() else {
            p.err(
                i,
                col,
                format!("everything in a Spreadsheet is inside a sheet: {SHEET_FORM}."),
            );
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
        if t.starts_with("<merge") {
            let prev: Vec<String> = sheet
                .items
                .iter()
                .filter_map(|it| match it {
                    SheetItem::Merge(m) => Some(m.range.clone()),
                    _ => None,
                })
                .collect();
            if let Some(m) = merge_tag(&mut p, i, &prev) {
                sheet.items.push(SheetItem::Merge(m));
            }
            i += 1;
            continue;
        }
        if t.starts_with("<hyperlink") {
            let prev: Vec<String> = sheet
                .items
                .iter()
                .filter_map(|it| match it {
                    SheetItem::Hyperlink(h) => Some(h.range.clone()),
                    _ => None,
                })
                .collect();
            if let Some(h) = hyperlink_tag(&mut p, i, &prev) {
                sheet.items.push(SheetItem::Hyperlink(h));
            }
            i += 1;
            continue;
        }
        if t.starts_with("<note") {
            let prev: Vec<String> = sheet
                .items
                .iter()
                .filter_map(|it| match it {
                    SheetItem::Note(n) => Some(n.range.clone()),
                    _ => None,
                })
                .collect();
            if let Some(n) = note_tag(&mut p, i, &prev) {
                sheet.items.push(SheetItem::Note(n));
            }
            i += 1;
            continue;
        }
        if t.starts_with("<validation") {
            let n = sheet
                .items
                .iter()
                .filter(|it| matches!(it, SheetItem::Validation(_)))
                .count();
            if let Some(v) = validation_tag(&mut p, i, n) {
                sheet.items.push(SheetItem::Validation(v));
            }
            i += 1;
            continue;
        }
        if t.starts_with("<conditional-format") {
            let n = sheet
                .items
                .iter()
                .filter(|it| matches!(it, SheetItem::ConditionalFormat(_)))
                .count();
            if let Some(c) = conditional_format_tag(&mut p, i, n) {
                sheet.items.push(SheetItem::ConditionalFormat(Box::new(c)));
            }
            i += 1;
            continue;
        }
        if t.starts_with("<chart") {
            if let Some(c) = chart_tag(&mut p, i) {
                sheet.items.push(SheetItem::Chart(c));
            }
            i += 1;
            continue;
        }
        if t.starts_with("<shape") {
            if let Some(s) = shape_tag(&mut p, i, &mut n_shapes) {
                sheet.items.push(SheetItem::Shape(s));
            }
            i += 1;
            continue;
        }
        if t.starts_with("<group") {
            let (g, next) = group_block(&mut p, i, &mut n_shapes);
            if let Some(g) = g {
                sheet.items.push(SheetItem::Group(g));
            }
            i = next;
            continue;
        }
        if t.starts_with("</group") {
            p.err(
                i,
                col,
                format!("</group> closes no group; a group is {GROUP_FORM}."),
            );
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
            p.err(i, col, format!("a sheet holds only table blocks ({TABLE_FORM}), merge lines ({MERGE_FORM}), hyperlink lines ({HYPERLINK_FORM}), note lines ({NOTE_FORM}), validation lines ({VALIDATION_FORM}), conditional-format lines ({CONDITIONAL_FORMAT_FORM}), shape lines ({SHAPE_FORM}), group blocks ({GROUP_FORM}), chart lines ({CHART_FORM}), placeholders ({KEEP_FORM}) and the format lines of its cells."));
        }
        i += 1;
    }
    if let Some((at, s)) = open {
        p.err(
            at,
            1,
            format!("sheet \"{}\" is not closed by a line </sheet>.", s.name),
        );
    }
    check_names(&mut p, &sheets, &keeps);
    if !p.errors.is_empty() {
        p.errors.sort_by_key(|d| (d.line, d.col));
        return Err(p.errors);
    }
    Ok(Spreadsheet {
        front,
        default_format,
        sheets,
    })
}

/// The attributes of a single-line tag at line `i`, checked against `allowed`.
fn tag_attrs(
    p: &mut Parser<'_>,
    i: usize,
    name: &str,
    form: &str,
    allowed: &[&str],
) -> Option<Vec<(String, String)>> {
    let line = &p.lines[i];
    let lead = line.text.len() - line.text.trim_start().len();
    let src = chars_of(line, lead);
    let col = lead + 1;
    let Some((tag, end)) = parse_tag(&src, 0) else {
        p.err(
            i,
            col,
            format!("this line is not a well-formed tag; it is {form}."),
        );
        return None;
    };
    if tag.name != name || tag.closing {
        p.err(i, col, format!("expected {form}."));
        return None;
    }
    if src[end..].iter().any(|s| !s.2.is_whitespace()) {
        p.err(
            i,
            src.get(end).map_or(col, |s| s.1),
            format!("nothing follows the tag on its line; it is {form}."),
        );
        return None;
    }
    let mut out = vec![];
    for (k, v, c) in &tag.attrs {
        if !allowed.contains(&k.as_str()) {
            p.err(
                i,
                *c,
                format!("<{name}> has no attribute \"{k}\"; it is {form}."),
            );
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
    Some(SheetDecl {
        name: name.to_string(),
        range,
        formats: vec![],
        items: vec![],
    })
}

fn is_range(r: &str) -> bool {
    let cell = |s: &str| {
        let k = s.find(|c: char| !c.is_ascii_uppercase()).unwrap_or(s.len());
        k > 0
            && k <= 3
            && k < s.len()
            && s[k..].bytes().all(|b| b.is_ascii_digit())
            && !s[k..].starts_with('0')
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
    let inner = t
        .strip_prefix('|')?
        .strip_suffix('|')
        .filter(|_| t.len() >= 2)?;
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
    !cells.is_empty()
        && cells
            .iter()
            .all(|c| c.trim().chars().all(|x| x == '-' || x == ':') && c.contains('-'))
}

fn table_block(p: &mut Parser<'_>, i: usize) -> (Option<TableDecl>, usize) {
    let n = p.lines.len();
    let close = (i + 1..n).find(|&k| {
        let t = p.lines[k].text.trim();
        t.starts_with("</table")
            || t.starts_with("<table")
            || t.starts_with("<sheet")
            || t.starts_with("</sheet")
    });
    let next = match close {
        Some(k) if p.lines[k].text.trim().starts_with("</table") => k + 1,
        Some(k) => {
            p.err(
                i,
                1,
                format!("this table is not closed by a line </table>; a table is {TABLE_FORM}."),
            );
            return (None, k);
        }
        None => {
            p.err(
                i,
                1,
                format!("this table is not closed by a line </table>; a table is {TABLE_FORM}."),
            );
            return (None, n);
        }
    };
    let close = next - 1;
    let closer = p.lines[close].text.trim();
    if closer != "</table>" {
        p.err(close, 1, "the line closing a table is exactly </table>.");
    }
    let Some(attrs) = tag_attrs(p, i, "table", TABLE_FORM, &["name", "range"]) else {
        return (None, next);
    };
    let (Some(name), Some(range)) = (get(&attrs, "name"), get(&attrs, "range")) else {
        p.err(
            i,
            1,
            format!("<table> has a name and a range: {TABLE_FORM}."),
        );
        return (None, next);
    };
    if let Some(m) = table_name_problem(name) {
        p.err(i, 1, format!("table \"{name}\": {m}."));
        return (None, next);
    }
    if !is_range(range) {
        p.err(
            i,
            1,
            format!("table \"{name}\": range=\"{range}\" is not an A1 range such as A1:F20."),
        );
        return (None, next);
    }
    let body: Vec<usize> = (i + 1..close)
        .filter(|&k| !p.lines[k].text.trim().is_empty())
        .collect();
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
    if !body
        .get(1)
        .and_then(|&k| split_pipe_row(p.lines[k].text))
        .is_some_and(|d| is_delim_row(&d))
    {
        let at = body.get(1).copied().unwrap_or(body[0]);
        p.err(
            at,
            1,
            "the second line of a table block is |---|---|---|---|.",
        );
        return (None, next);
    }
    let mut columns: Vec<ColumnDecl> = vec![];
    let before = p.errors.len();
    for &k in &body[2..] {
        let Some(cells) = split_pipe_row(p.lines[k].text) else {
            p.err(
                k,
                1,
                "each line of a table block is one column: | name | type | format | formula |.",
            );
            continue;
        };
        if cells.len() < 3 || cells.len() > 4 {
            p.err(k, 1, format!("a column line has four cells — | name | type | format | formula | — the formula cell empty for a column that is not a formula column; this line has {}.", cells.len()));
            continue;
        }
        let (cname, ty, format) = (&cells[0], &cells[1].trim().to_string(), &cells[2]);
        let formula = cells
            .get(3)
            .map(|f| f.trim().to_string())
            .unwrap_or_default();
        if cname.trim().is_empty() {
            p.err(k, 1, "a column has a name (its header cell).");
            continue;
        }
        if !COLUMN_TYPES.contains(&ty.as_str()) {
            p.err(
                k,
                1,
                format!(
                    "column \"{cname}\": the type is one of {}, not \"{ty}\".",
                    quoted_list(COLUMN_TYPES)
                ),
            );
            continue;
        }
        if format.trim().is_empty() {
            p.err(
                k,
                1,
                format!("column \"{cname}\" has a number format (General for none; @ for text)."),
            );
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
        columns.push(ColumnDecl {
            name: cname.clone(),
            ty: ty.clone(),
            format: format.clone(),
            formula,
        });
    }
    if columns.is_empty() && p.errors.len() == before {
        p.err(
            i,
            1,
            format!("table \"{name}\" has no columns; a table has one line per column."),
        );
    }
    (
        (p.errors.len() == before).then(|| TableDecl {
            name: name.to_string(),
            range: range.to_string(),
            columns,
        }),
        next,
    )
}

fn quoted_list(v: &[&str]) -> String {
    quoted(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn merge_tag(p: &mut Parser<'_>, i: usize, prev: &[String]) -> Option<MergeDecl> {
    let attrs = tag_attrs(p, i, "merge", MERGE_FORM, &["range"])?;
    let Some(raw) = get(&attrs, "range") else {
        p.err(i, 1, format!("<merge> has a range: {MERGE_FORM}."));
        return None;
    };
    if prev.len() >= MAX_MERGE_LINES {
        p.err(
            i,
            1,
            format!(
                "this sheet already has {MAX_MERGE_LINES} merged ranges; at most {MAX_MERGE_LINES} are written as {MERGE_FORM} lines (a sheet with more keeps them as a placeholder)."
            ),
        );
        return None;
    }
    let range = match parse_merge_range(raw) {
        Ok(r) => r,
        Err(m) => {
            p.err(i, 1, m);
            return None;
        }
    };
    let Some(rect) = merge_rect(&range) else {
        p.err(
            i,
            1,
            format!("range=\"{raw}\" is not an A1 range of two or more cells such as A1:B2."),
        );
        return None;
    };
    for old in prev {
        if let Some(o) = merge_rect(old) {
            if rect.overlaps(&o) {
                p.err(
                    i,
                    1,
                    format!(
                        "range=\"{range}\" overlaps {old}; merged ranges on a sheet do not overlap."
                    ),
                );
                return None;
            }
        }
    }
    Some(MergeDecl { range })
}

fn hyperlink_tag(p: &mut Parser<'_>, i: usize, prev: &[String]) -> Option<HyperlinkDecl> {
    let attrs = tag_attrs(
        p,
        i,
        "hyperlink",
        HYPERLINK_FORM,
        &["range", "target", "location", "display", "tooltip"],
    )?;
    let Some(raw) = get(&attrs, "range") else {
        p.err(i, 1, format!("<hyperlink> has a range: {HYPERLINK_FORM}."));
        return None;
    };
    if prev.len() >= MAX_HYPERLINK_LINES {
        p.err(
            i,
            1,
            format!(
                "this sheet already has {MAX_HYPERLINK_LINES} hyperlinks; at most {MAX_HYPERLINK_LINES} are written as {HYPERLINK_FORM} lines (a sheet with more keeps them as a placeholder)."
            ),
        );
        return None;
    }
    let range = match parse_sheet_range(raw) {
        Ok(r) => r,
        Err(m) => {
            p.err(i, 1, m);
            return None;
        }
    };
    let target = get(&attrs, "target").map(str::to_string);
    let location = get(&attrs, "location").map(str::to_string);
    if target.as_ref().is_some_and(|s| s.is_empty()) {
        p.err(
            i,
            1,
            format!("target is empty; an external hyperlink is {HYPERLINK_FORM}."),
        );
        return None;
    }
    if location.as_ref().is_some_and(|s| s.is_empty()) {
        p.err(
            i,
            1,
            "location is empty; an internal hyperlink names a sheet cell or a defined name, such as Sheet2!A1.",
        );
        return None;
    }
    if target.is_none() && location.is_none() {
        p.err(
            i,
            1,
            format!("<hyperlink> has a target URL or a location: {HYPERLINK_FORM}."),
        );
        return None;
    }
    if overlap_error(p, i, &range, prev, "hyperlinks") {
        return None;
    }
    Some(HyperlinkDecl {
        range,
        target,
        location,
        display: opt_attr(&attrs, "display"),
        tooltip: opt_attr(&attrs, "tooltip"),
    })
}

fn note_tag(p: &mut Parser<'_>, i: usize, prev: &[String]) -> Option<NoteDecl> {
    let attrs = tag_attrs(p, i, "note", NOTE_FORM, &["range", "author", "text"])?;
    let (Some(raw), Some(author), Some(text)) = (
        get(&attrs, "range"),
        get(&attrs, "author"),
        get(&attrs, "text"),
    ) else {
        p.err(
            i,
            1,
            format!("<note> has a range, an author and text: {NOTE_FORM}."),
        );
        return None;
    };
    if prev.len() >= MAX_NOTE_LINES {
        p.err(
            i,
            1,
            format!(
                "this sheet already has {MAX_NOTE_LINES} notes; at most {MAX_NOTE_LINES} are written as {NOTE_FORM} lines (a sheet with more keeps them as a placeholder)."
            ),
        );
        return None;
    }
    let range = match parse_sheet_range(raw) {
        Ok(r) => r,
        Err(m) => {
            p.err(i, 1, m);
            return None;
        }
    };
    if overlap_error(p, i, &range, prev, "notes") {
        return None;
    }
    Some(NoteDecl {
        range,
        author: author.to_string(),
        text: text.to_string(),
    })
}

fn shape_tag(p: &mut Parser<'_>, i: usize, n_shapes: &mut usize) -> Option<ShapeDecl> {
    let line = &p.lines[i];
    let lead = line.text.len() - line.text.trim_start().len();
    let src = chars_of(line, lead);
    let col = lead + 1;
    let Some((tag, end)) = parse_tag(&src, 0) else {
        p.err(
            i,
            col,
            format!("this line is not a well-formed tag; it is {SHAPE_FORM}."),
        );
        return None;
    };
    if tag.name != "shape" || tag.closing {
        p.err(i, col, format!("expected {SHAPE_FORM}."));
        return None;
    }
    let mut attrs = vec![];
    for (k, v, c) in &tag.attrs {
        if !matches!(k.as_str(), "name" | "anchor") {
            p.err(
                i,
                *c,
                format!("<shape> has no attribute \"{k}\"; it is {SHAPE_FORM}."),
            );
            return None;
        }
        if attrs.iter().any(|(x, _): &(String, String)| x == k) {
            p.err(i, *c, format!("\"{k}\" appears twice in <shape>."));
            return None;
        }
        attrs.push((k.clone(), v.clone()));
    }
    let (Some(name), Some(raw)) = (get(&attrs, "name"), get(&attrs, "anchor")) else {
        p.err(
            i,
            col,
            format!("<shape> has a name and an anchor: {SHAPE_FORM}."),
        );
        return None;
    };
    if name.is_empty() {
        p.err(i, col, format!("name is empty; a shape is {SHAPE_FORM}."));
        return None;
    }
    if *n_shapes >= MAX_SHAPE_LINES {
        p.err(
            i,
            col,
            format!(
                "this sheet already has {MAX_SHAPE_LINES} shapes; at most {MAX_SHAPE_LINES} are written as {SHAPE_FORM} lines (a sheet with more keeps them as a placeholder)."
            ),
        );
        return None;
    }
    let anchor = match parse_sheet_range(raw) {
        Ok(r) => r,
        Err(m) => {
            p.err(i, col, m.replace("range=", "anchor="));
            return None;
        }
    };
    let text = if tag.self_closing {
        if src[end..].iter().any(|s| !s.2.is_whitespace()) {
            p.err(
                i,
                src.get(end).map_or(col, |s| s.1),
                format!("nothing follows the tag on its line; it is {SHAPE_FORM}."),
            );
            return None;
        }
        String::new()
    } else {
        let rest = &src[end..];
        let mut k = 0;
        let mut close = None;
        while k < rest.len() {
            if rest[k].2 == '<' {
                match parse_tag(rest, k) {
                    Some((ct, cend)) if ct.closing && ct.name == "shape" => {
                        close = Some((k, cend, ct.col));
                        break;
                    }
                    Some(_) => {
                        p.err(
                            i,
                            rest[k].1,
                            "shape text cannot hold a tag; write < as &lt;.",
                        );
                        return None;
                    }
                    None => {
                        p.err(
                            i,
                            rest[k].1,
                            format!("<shape> is not closed; it is {SHAPE_FORM}."),
                        );
                        return None;
                    }
                }
            }
            k += 1;
        }
        let Some((ck, cend, _)) = close else {
            p.err(
                i,
                col,
                format!("<shape> is not closed; it is {SHAPE_FORM}."),
            );
            return None;
        };
        if rest[cend..].iter().any(|s| !s.2.is_whitespace()) {
            p.err(
                i,
                rest.get(cend).map_or(col, |s| s.1),
                format!("nothing follows </shape> on its line; it is {SHAPE_FORM}."),
            );
            return None;
        }
        let raw: String = rest[..ck].iter().map(|s| s.2).collect();
        decode_attr(&raw)
    };
    *n_shapes += 1;
    Some(ShapeDecl {
        name: name.to_string(),
        anchor,
        text,
    })
}

fn group_open(p: &mut Parser<'_>, i: usize) -> Option<String> {
    let attrs = tag_open(p, i, "group", GROUP_FORM, &["name"])?;
    let Some(name) = get(&attrs, "name") else {
        p.err(i, 1, format!("<group> has a name: {GROUP_FORM}."));
        return None;
    };
    if name.is_empty() {
        p.err(i, 1, format!("name is empty; a group is {GROUP_FORM}."));
        return None;
    }
    Some(name.to_string())
}

/// `<name attr="…">` on its own line, not self-closing.
fn tag_open(
    p: &mut Parser<'_>,
    i: usize,
    name: &str,
    form: &str,
    allowed: &[&str],
) -> Option<Vec<(String, String)>> {
    let line = &p.lines[i];
    let lead = line.text.len() - line.text.trim_start().len();
    let src = chars_of(line, lead);
    let col = lead + 1;
    let Some((tag, end)) = parse_tag(&src, 0) else {
        p.err(
            i,
            col,
            format!("this line is not a well-formed tag; it is {form}."),
        );
        return None;
    };
    if tag.name != name || tag.closing {
        p.err(i, col, format!("expected {form}."));
        return None;
    }
    if tag.self_closing {
        p.err(
            i,
            col,
            format!("<{name}> is not closed by />; it is {form}."),
        );
        return None;
    }
    if src[end..].iter().any(|s| !s.2.is_whitespace()) {
        p.err(
            i,
            src.get(end).map_or(col, |s| s.1),
            format!("nothing follows the tag on its line; it is {form}."),
        );
        return None;
    }
    let mut out = vec![];
    for (k, v, c) in &tag.attrs {
        if !allowed.contains(&k.as_str()) {
            p.err(
                i,
                *c,
                format!("<{name}> has no attribute \"{k}\"; it is {form}."),
            );
            return None;
        }
        if out.iter().any(|(x, _): &(String, String)| x == k) {
            p.err(i, *c, format!("\"{k}\" appears twice in <{name}>."));
            return None;
        }
        out.push((k.clone(), v.clone()));
    }
    Some(out)
}

fn group_block(p: &mut Parser<'_>, i: usize, n_shapes: &mut usize) -> (Option<GroupDecl>, usize) {
    let n = p.lines.len();
    let Some(name) = group_open(p, i) else {
        return (None, i + 1);
    };
    let mut items = vec![];
    let mut j = i + 1;
    let mut closed = false;
    while j < n {
        let t = p.lines[j].text.trim();
        if t.is_empty() {
            j += 1;
            continue;
        }
        if t.starts_with("</group") {
            let line = &p.lines[j];
            let lead = line.text.len() - line.text.trim_start().len();
            let src = chars_of(line, lead);
            match parse_tag(&src, 0) {
                Some((tag, end)) if tag.name == "group" && tag.closing && !tag.self_closing => {
                    if src[end..].iter().any(|s| !s.2.is_whitespace()) {
                        p.err(
                            j,
                            src.get(end).map_or(lead + 1, |s| s.1),
                            format!(
                                "nothing follows </group> on its line; a group is {GROUP_FORM}."
                            ),
                        );
                    }
                }
                _ => p.err(
                    j,
                    lead + 1,
                    format!("expected </group>; a group is {GROUP_FORM}."),
                ),
            }
            j += 1;
            closed = true;
            break;
        }
        if t.starts_with("<group") {
            let (g, next) = group_block(p, j, n_shapes);
            if let Some(g) = g {
                items.push(DrawingItem::Group(g));
            }
            j = next;
            continue;
        }
        if t.starts_with("<shape") {
            if let Some(s) = shape_tag(p, j, n_shapes) {
                items.push(DrawingItem::Shape(s));
            }
            j += 1;
            continue;
        }
        p.err(
            j,
            1,
            format!("a group holds shape lines ({SHAPE_FORM}) and nested groups ({GROUP_FORM})."),
        );
        j += 1;
    }
    if !closed {
        p.err(
            i,
            1,
            format!("group \"{name}\" is not closed by a line </group>."),
        );
        return (None, j);
    }
    if items.is_empty() {
        p.err(
            i,
            1,
            format!("group \"{name}\" is empty; a group is {GROUP_FORM}."),
        );
        return (None, j);
    }
    (Some(GroupDecl { name, items }), j)
}

fn opt_attr(attrs: &[(String, String)], k: &str) -> Option<String> {
    get(attrs, k).filter(|s| !s.is_empty()).map(str::to_string)
}

const VALIDATION_ATTRS: &[&str] = &[
    "range",
    "type",
    "operator",
    "formula1",
    "formula2",
    "allowblank",
    "showinputmessage",
    "showerrormessage",
    "prompttitle",
    "prompt",
    "errortitle",
    "error",
    "errorstyle",
];

fn validation_tag(p: &mut Parser<'_>, i: usize, have: usize) -> Option<ValidationDecl> {
    let attrs = tag_attrs(p, i, "validation", VALIDATION_FORM, VALIDATION_ATTRS)?;
    let (Some(raw), Some(ty)) = (get(&attrs, "range"), get(&attrs, "type")) else {
        p.err(
            i,
            1,
            format!("<validation> has a range and a type: {VALIDATION_FORM}."),
        );
        return None;
    };
    if have >= MAX_VALIDATION_LINES {
        p.err(
            i,
            1,
            format!(
                "this sheet already has {MAX_VALIDATION_LINES} data validations; at most {MAX_VALIDATION_LINES} are written as {VALIDATION_FORM} lines (a sheet with more keeps them as a placeholder)."
            ),
        );
        return None;
    }
    if !VALIDATION_TYPES.contains(&ty) {
        p.err(
            i,
            1,
            format!(
                "validation type \"{ty}\" is not one of {}.",
                quoted_list(VALIDATION_TYPES)
            ),
        );
        return None;
    }
    let range = match parse_sqref_text(raw) {
        Ok(r) => r,
        Err(m) => {
            p.err(i, 1, m);
            return None;
        }
    };
    let operator = match get(&attrs, "operator") {
        None => None,
        Some("between") => None,
        Some(op) if VALIDATION_OPERATORS.contains(&op) => Some(op.to_string()),
        Some(op) => {
            p.err(
                i,
                1,
                format!(
                    "operator \"{op}\" is not one of {}.",
                    quoted_list(VALIDATION_OPERATORS)
                ),
            );
            return None;
        }
    };
    let error_style = match get(&attrs, "errorstyle") {
        None | Some("stop") => None,
        Some(s) if VALIDATION_ERROR_STYLES.contains(&s) => Some(s.to_string()),
        Some(s) => {
            p.err(
                i,
                1,
                format!(
                    "errorStyle \"{s}\" is not one of {}.",
                    quoted_list(VALIDATION_ERROR_STYLES)
                ),
            );
            return None;
        }
    };
    Some(ValidationDecl {
        range,
        ty: ty.to_string(),
        operator,
        formula1: opt_attr(&attrs, "formula1"),
        formula2: opt_attr(&attrs, "formula2"),
        allow_blank: parse_flag_attr(p, i, &attrs, "allowblank")? == Some(true),
        show_input_message: match parse_flag_attr(p, i, &attrs, "showinputmessage")? {
            Some(true) | None => None,
            Some(false) => Some(false),
        },
        show_error_message: match parse_flag_attr(p, i, &attrs, "showerrormessage")? {
            Some(true) | None => None,
            Some(false) => Some(false),
        },
        prompt_title: opt_attr(&attrs, "prompttitle"),
        prompt: opt_attr(&attrs, "prompt"),
        error_title: opt_attr(&attrs, "errortitle"),
        error: opt_attr(&attrs, "error"),
        error_style,
    })
}

const CONDITIONAL_FORMAT_ATTRS: &[&str] = &[
    "range",
    "type",
    "operator",
    "formula",
    "formula2",
    "dxf",
    "stopiftrue",
    "min",
    "max",
    "fill",
];

fn conditional_format_tag(p: &mut Parser<'_>, i: usize, have: usize) -> Option<CondFormatDecl> {
    let attrs = tag_attrs(
        p,
        i,
        "conditional-format",
        CONDITIONAL_FORMAT_FORM,
        CONDITIONAL_FORMAT_ATTRS,
    )?;
    let (Some(raw), Some(ty)) = (get(&attrs, "range"), get(&attrs, "type")) else {
        p.err(
            i,
            1,
            format!("<conditional-format> has a range and a type: {CONDITIONAL_FORMAT_FORM}."),
        );
        return None;
    };
    if have >= MAX_CONDITIONAL_FORMAT_LINES {
        p.err(
            i,
            1,
            format!(
                "this sheet already has {MAX_CONDITIONAL_FORMAT_LINES} conditional formats; at most {MAX_CONDITIONAL_FORMAT_LINES} are written as {CONDITIONAL_FORMAT_FORM} lines (a sheet with more keeps them as a placeholder)."
            ),
        );
        return None;
    }
    if !CONDITIONAL_FORMAT_TYPES.contains(&ty) {
        p.err(
            i,
            1,
            format!(
                "conditional-format type \"{ty}\" is not one of {} (other rules stay a placeholder).",
                quoted_list(CONDITIONAL_FORMAT_TYPES)
            ),
        );
        return None;
    }
    let range = match parse_sqref_text(raw) {
        Ok(r) => r,
        Err(m) => {
            p.err(i, 1, m);
            return None;
        }
    };
    if ty == "dataBar" {
        return data_bar_tag(p, i, &attrs, range);
    }
    if get(&attrs, "min").is_some() || get(&attrs, "max").is_some() || get(&attrs, "fill").is_some()
    {
        p.err(
            i,
            1,
            format!(
                "min, max and fill belong on type=\"dataBar\", not type=\"{ty}\"; a {ty} rule is {CONDITIONAL_FORMAT_FORM}."
            ),
        );
        return None;
    }
    let operator = match (ty, get(&attrs, "operator")) {
        ("expression", Some(_)) => {
            p.err(
                i,
                1,
                "an expression rule has no operator; write type=\"expression\" formula=\"…\".",
            );
            return None;
        }
        (_, None) | (_, Some("between")) => None,
        (_, Some(op)) if VALIDATION_OPERATORS.contains(&op) => Some(op.to_string()),
        (_, Some(op)) => {
            p.err(
                i,
                1,
                format!(
                    "operator \"{op}\" is not one of {}.",
                    quoted_list(VALIDATION_OPERATORS)
                ),
            );
            return None;
        }
    };
    let dxf = match get(&attrs, "dxf") {
        None => CellFormat::default(),
        Some(s) => match CellFormat::parse(s) {
            Ok(f) => f,
            Err((_, m)) => {
                p.err(i, 1, format!("dxf: {m}"));
                return None;
            }
        },
    };
    Some(CondFormatDecl {
        range,
        ty: ty.to_string(),
        operator,
        formula: opt_attr(&attrs, "formula"),
        formula2: opt_attr(&attrs, "formula2"),
        dxf,
        stop_if_true: parse_flag_attr(p, i, &attrs, "stopiftrue")? == Some(true),
        min: None,
        max: None,
        fill: None,
    })
}

fn data_bar_tag(
    p: &mut Parser<'_>,
    i: usize,
    attrs: &[(String, String)],
    range: String,
) -> Option<CondFormatDecl> {
    for (k, label) in [
        ("operator", "operator"),
        ("formula", "formula"),
        ("formula2", "formula2"),
        ("dxf", "dxf"),
    ] {
        if get(attrs, k).is_some() {
            p.err(
                i,
                1,
                format!("a dataBar has min, max and fill, not {label}: {CONDITIONAL_FORMAT_FORM}."),
            );
            return None;
        }
    }
    let (Some(min_raw), Some(max_raw), Some(fill_raw)) =
        (get(attrs, "min"), get(attrs, "max"), get(attrs, "fill"))
    else {
        p.err(
            i,
            1,
            format!("a dataBar has min, max and fill: {CONDITIONAL_FORMAT_FORM}."),
        );
        return None;
    };
    let min = match Cfvo::parse(min_raw) {
        Ok(v) => v,
        Err(m) => {
            p.err(i, 1, format!("min: {m}"));
            return None;
        }
    };
    let max = match Cfvo::parse(max_raw) {
        Ok(v) => v,
        Err(m) => {
            p.err(i, 1, format!("max: {m}"));
            return None;
        }
    };
    let fill = match Color::parse(fill_raw) {
        Ok(c) if c.alpha.is_none() && c.is_writable() => c,
        Ok(c) => {
            p.err(
                i,
                1,
                format!(
                    "fill {c} cannot be written on a dataBar; write #RRGGBB or a theme colour (accent1)."
                ),
            );
            return None;
        }
        Err(m) => {
            p.err(i, 1, format!("fill: {m}"));
            return None;
        }
    };
    Some(CondFormatDecl {
        range,
        ty: "dataBar".into(),
        operator: None,
        formula: None,
        formula2: None,
        dxf: CellFormat::default(),
        stop_if_true: parse_flag_attr(p, i, attrs, "stopiftrue")? == Some(true),
        min: Some(min),
        max: Some(max),
        fill: Some(fill),
    })
}

fn parse_flag_attr(
    p: &mut Parser<'_>,
    i: usize,
    attrs: &[(String, String)],
    k: &str,
) -> Option<Option<bool>> {
    match get(attrs, k) {
        None => Some(None),
        Some("true" | "1") => Some(Some(true)),
        Some("false" | "0") => Some(Some(false)),
        Some(v) => {
            p.err(i, 1, format!("{k}=\"{v}\" is not true or false."));
            None
        }
    }
}

/// Canonical space-separated A1 ranges (`B2:A1 C3` → `A1:B2 C3`).
fn parse_sqref_text(s: &str) -> Result<String, String> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() {
        return Err("range is empty; write an A1 cell or range such as A1 or A1:B2.".into());
    }
    let mut out = Vec::with_capacity(parts.len());
    for p in parts {
        out.push(parse_sheet_range(p)?);
    }
    Ok(out.join(" "))
}

fn overlap_error(p: &mut Parser<'_>, i: usize, range: &str, prev: &[String], what: &str) -> bool {
    let Some(rect) = range_rect(range) else {
        return false;
    };
    for old in prev {
        if let Some(o) = range_rect(old) {
            if rect.overlaps(&o) {
                p.err(
                    i,
                    1,
                    format!("range=\"{range}\" overlaps {old}; {what} on a sheet do not overlap."),
                );
                return true;
            }
        }
    }
    false
}

/// Canonical A1 form of a merge (`B2:A1` → `A1:B2`). Two or more cells,
/// inside Excel's grid.
fn parse_merge_range(s: &str) -> Result<String, String> {
    let form = "an A1 range of two or more cells such as A1:B2 (Excel's last cell is XFD1048576)";
    let Some((a, b)) = s.split_once(':') else {
        return Err(format!("range=\"{s}\" is one cell; a merge is {form}."));
    };
    let (Some(p), Some(q)) = (parse_a1(a), parse_a1(b)) else {
        return Err(format!("range=\"{s}\" is not {form}."));
    };
    let first_col = p.0.min(q.0);
    let last_col = p.0.max(q.0);
    let first_row = p.1.min(q.1);
    let last_row = p.1.max(q.1);
    if first_col == last_col && first_row == last_row {
        return Err(format!("range=\"{s}\" is one cell; a merge is {form}."));
    }
    Ok(format!(
        "{}{}:{}{}",
        col_letters(first_col),
        first_row,
        col_letters(last_col),
        last_row
    ))
}

/// Canonical A1 form of a cell or range (`B2:A1` → `A1:B2`, `A1:A1` → `A1`).
fn parse_sheet_range(s: &str) -> Result<String, String> {
    let form = "an A1 cell or range such as A1 or A1:B2 (Excel's last cell is XFD1048576)";
    match s.split_once(':') {
        None => {
            let Some(p) = parse_a1(s) else {
                return Err(format!("range=\"{s}\" is not {form}."));
            };
            Ok(format!("{}{}", col_letters(p.0), p.1))
        }
        Some((a, b)) => {
            let (Some(p), Some(q)) = (parse_a1(a), parse_a1(b)) else {
                return Err(format!("range=\"{s}\" is not {form}."));
            };
            let first_col = p.0.min(q.0);
            let last_col = p.0.max(q.0);
            let first_row = p.1.min(q.1);
            let last_row = p.1.max(q.1);
            if first_col == last_col && first_row == last_row {
                return Ok(format!("{}{}", col_letters(first_col), first_row));
            }
            Ok(format!(
                "{}{}:{}{}",
                col_letters(first_col),
                first_row,
                col_letters(last_col),
                last_row
            ))
        }
    }
}

fn parse_a1(s: &str) -> Option<(u32, u32)> {
    let s = s.trim();
    let s = s.strip_prefix('$').unwrap_or(s);
    let k = s.find(|c: char| !c.is_ascii_alphabetic())?;
    let (letters, rest) = s.split_at(k);
    let rest = rest.strip_prefix('$').unwrap_or(rest);
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) || rest.starts_with('0') {
        return None;
    }
    let row: u32 = rest.parse().ok()?;
    (1..=MAX_ROW)
        .contains(&row)
        .then_some((col_index(letters)?, row))
}

fn col_index(letters: &str) -> Option<u32> {
    if letters.is_empty() || letters.len() > 3 {
        return None;
    }
    let mut n: u32 = 0;
    for b in letters.bytes() {
        if !b.is_ascii_alphabetic() {
            return None;
        }
        n = n * 26 + u32::from(b.to_ascii_uppercase() - b'A' + 1);
    }
    (1..=MAX_COL).contains(&n).then_some(n - 1)
}

fn col_letters(col: u32) -> String {
    let mut s = Vec::new();
    let mut n = col + 1;
    while n > 0 {
        let r = (n - 1) % 26;
        s.push(b'A' + r as u8);
        n = (n - 1) / 26;
    }
    s.reverse();
    String::from_utf8(s).unwrap()
}

struct MergeRect {
    c0: u32,
    r0: u32,
    c1: u32,
    r1: u32,
}

impl MergeRect {
    fn overlaps(&self, o: &MergeRect) -> bool {
        self.c0 <= o.c1 && o.c0 <= self.c1 && self.r0 <= o.r1 && o.r0 <= self.r1
    }
}

fn merge_rect(s: &str) -> Option<MergeRect> {
    let (a, b) = s.split_once(':')?;
    range_rect(&format!("{a}:{b}")).filter(|r| r.c0 != r.c1 || r.r0 != r.r1)
}

fn range_rect(s: &str) -> Option<MergeRect> {
    match s.split_once(':') {
        Some((a, b)) => {
            let p = parse_a1(a)?;
            let q = parse_a1(b)?;
            Some(MergeRect {
                c0: p.0.min(q.0),
                r0: p.1.min(q.1),
                c1: p.0.max(q.0),
                r1: p.1.max(q.1),
            })
        }
        None => {
            let p = parse_a1(s)?;
            Some(MergeRect {
                c0: p.0,
                r0: p.1,
                c1: p.0,
                r1: p.1,
            })
        }
    }
}

fn chart_tag(p: &mut Parser<'_>, i: usize) -> Option<ChartDecl> {
    let attrs = tag_attrs(p, i, "chart", CHART_FORM, &["type", "data", "title"])?;
    let (Some(ty), Some(data)) = (get(&attrs, "type"), get(&attrs, "data")) else {
        p.err(i, 1, format!("<chart> has a type and data: {CHART_FORM}."));
        return None;
    };
    if !CHART_TYPES.contains(&ty) {
        p.err(
            i,
            1,
            format!(
                "chart type \"{ty}\" is not one of {}.",
                quoted_list(CHART_TYPES)
            ),
        );
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
    Some(ChartDecl {
        ty: ty.into(),
        data: data.into(),
        title: get(&attrs, "title").map(str::to_string),
    })
}

fn keep_tag(p: &mut Parser<'_>, i: usize) -> Option<Keep> {
    let attrs = tag_attrs(p, i, "keep", KEEP_FORM, &["id", "kind", "summary"])?;
    match (
        get(&attrs, "id"),
        get(&attrs, "kind"),
        get(&attrs, "summary"),
    ) {
        (Some(id), Some(kind), Some(summary)) => Some(Keep {
            id: id.into(),
            kind: kind.into(),
            summary: summary.into(),
        }),
        _ => {
            p.err(i, 1, format!("a placeholder keeps its id, kind and summary as they are in the file: {KEEP_FORM}."));
            None
        }
    }
}

fn check_names(p: &mut Parser<'_>, sheets: &[SheetDecl], keeps: &[(usize, usize, Keep)]) {
    let line_of = |p: &Parser<'_>, pat: &str| {
        (0..p.lines.len())
            .find(|&k| p.lines[k].text.contains(pat))
            .unwrap_or(0)
    };
    for (k, s) in sheets.iter().enumerate() {
        if sheets[..k]
            .iter()
            .any(|o| o.name.to_lowercase() == s.name.to_lowercase())
        {
            let at = line_of(p, &format!("name=\"{}\"", s.name));
            p.err(
                at,
                1,
                format!(
                    "two sheets are named \"{}\"; sheet names are unique (case does not count).",
                    s.name
                ),
            );
        }
    }
    let tables: Vec<&TableDecl> = sheets
        .iter()
        .flat_map(|s| {
            s.items.iter().filter_map(|i| {
                if let SheetItem::Table(t) = i {
                    Some(t)
                } else {
                    None
                }
            })
        })
        .collect();
    for (k, t) in tables.iter().enumerate() {
        if tables[..k]
            .iter()
            .any(|o| o.name.to_lowercase() == t.name.to_lowercase())
        {
            let at = line_of(p, &format!("name=\"{}\"", t.name));
            p.err(
                at,
                1,
                format!(
                    "two tables are named \"{}\"; table names are unique in the workbook.",
                    t.name
                ),
            );
        }
    }
    let known = p.names.keeps.clone();
    for (k, (line, col, keep)) in keeps.iter().enumerate() {
        if keeps[..k].iter().any(|o| o.2.id == keep.id) {
            p.err(
                *line,
                *col,
                format!(
                    "placeholder id=\"{}\" appears twice; keep each placeholder once.",
                    keep.id
                ),
            );
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
    if let Some(d) = &s.default_format {
        out.push(String::new());
        out.push(cellfmt::default_line(d));
    }
    for sh in &s.sheets {
        out.push(String::new());
        out.push(sheet_open(sh));
        if !sh.formats.is_empty() {
            out.push(String::new());
            out.extend(sh.formats.iter().map(FormatLine::line));
        }
        let mut i = 0;
        while i < sh.items.len() {
            out.push(String::new());
            match &sh.items[i] {
                SheetItem::Merge(_) => {
                    while i < sh.items.len() {
                        match &sh.items[i] {
                            SheetItem::Merge(m) => {
                                out.push(merge_line(m));
                                i += 1;
                            }
                            _ => break,
                        }
                    }
                }
                SheetItem::Hyperlink(_) => {
                    while i < sh.items.len() {
                        match &sh.items[i] {
                            SheetItem::Hyperlink(h) => {
                                out.push(hyperlink_line(h));
                                i += 1;
                            }
                            _ => break,
                        }
                    }
                }
                SheetItem::Note(_) => {
                    while i < sh.items.len() {
                        match &sh.items[i] {
                            SheetItem::Note(n) => {
                                out.push(note_line(n));
                                i += 1;
                            }
                            _ => break,
                        }
                    }
                }
                SheetItem::Validation(_) => {
                    while i < sh.items.len() {
                        match &sh.items[i] {
                            SheetItem::Validation(v) => {
                                out.push(validation_line(v));
                                i += 1;
                            }
                            _ => break,
                        }
                    }
                }
                SheetItem::ConditionalFormat(_) => {
                    while i < sh.items.len() {
                        match &sh.items[i] {
                            SheetItem::ConditionalFormat(c) => {
                                out.push(conditional_format_line(c));
                                i += 1;
                            }
                            _ => break,
                        }
                    }
                }
                SheetItem::Shape(_) | SheetItem::Group(_) => {
                    while i < sh.items.len() {
                        match &sh.items[i] {
                            SheetItem::Shape(s) => {
                                out.push(shape_line(s));
                                i += 1;
                            }
                            SheetItem::Group(g) => {
                                out.extend(group_lines(g));
                                i += 1;
                            }
                            _ => break,
                        }
                    }
                }
                SheetItem::Table(t) => {
                    out.extend(table_lines(t));
                    i += 1;
                }
                SheetItem::Chart(c) => {
                    out.push(chart_line(c));
                    i += 1;
                }
                SheetItem::Keep(k) => {
                    out.push(format!(
                        "<keep id=\"{}\" kind=\"{}\" summary=\"{}\"/>",
                        attr(&k.id),
                        attr(&k.kind),
                        attr(&k.summary)
                    ));
                    i += 1;
                }
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
    s.replace('|', "\\|")
        .replace("\r\n", "<br/>")
        .replace(['\n', '\r'], "<br/>")
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
        out.push(row_line(&[
            pipe_cell(&c.name),
            c.ty.clone(),
            pipe_cell(&c.format),
            pipe_cell(&c.formula),
        ]));
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

pub fn merge_line(m: &MergeDecl) -> String {
    format!("<merge range=\"{}\"/>", attr(&m.range))
}

pub fn shape_line(s: &ShapeDecl) -> String {
    let open = format!(
        "<shape name=\"{}\" anchor=\"{}\"",
        attr(&s.name),
        attr(&s.anchor)
    );
    if s.text.is_empty() {
        format!("{open}/>")
    } else {
        format!("{open}>{}</shape>", shape_body(&s.text))
    }
}

pub fn group_lines(g: &GroupDecl) -> Vec<String> {
    let mut out = vec![format!("<group name=\"{}\">", attr(&g.name))];
    for it in &g.items {
        match it {
            DrawingItem::Shape(s) => out.push(shape_line(s)),
            DrawingItem::Group(inner) => out.extend(group_lines(inner)),
        }
    }
    out.push("</group>".into());
    out
}

fn shape_body(s: &str) -> String {
    attr(s)
        .replace('>', "&gt;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;")
}

pub fn hyperlink_line(h: &HyperlinkDecl) -> String {
    let mut s = format!("<hyperlink range=\"{}\"", attr(&h.range));
    if let Some(t) = &h.target {
        s.push_str(&format!(" target=\"{}\"", attr(t)));
    }
    if let Some(l) = &h.location {
        s.push_str(&format!(" location=\"{}\"", attr(l)));
    }
    if let Some(d) = &h.display {
        s.push_str(&format!(" display=\"{}\"", attr(d)));
    }
    if let Some(t) = &h.tooltip {
        s.push_str(&format!(" tooltip=\"{}\"", attr(t)));
    }
    s + "/>"
}

pub fn note_line(n: &NoteDecl) -> String {
    format!(
        "<note range=\"{}\" author=\"{}\" text=\"{}\"/>",
        attr(&n.range),
        attr(&n.author),
        attr_text(&n.text)
    )
}

fn push_opt(s: &mut String, k: &str, v: Option<&str>) {
    if let Some(v) = v {
        s.push_str(&format!(" {k}=\"{}\"", attr(v)));
    }
}

fn push_bool_true(s: &mut String, k: &str, on: bool) {
    if on {
        s.push_str(&format!(" {k}=\"true\""));
    }
}

fn push_bool_false(s: &mut String, k: &str, v: Option<bool>) {
    if v == Some(false) {
        s.push_str(&format!(" {k}=\"false\""));
    }
}

pub fn validation_line(v: &ValidationDecl) -> String {
    let mut s = format!(
        "<validation range=\"{}\" type=\"{}\"",
        attr(&v.range),
        attr(&v.ty)
    );
    push_opt(&mut s, "operator", v.operator.as_deref());
    push_opt(&mut s, "formula1", v.formula1.as_deref());
    push_opt(&mut s, "formula2", v.formula2.as_deref());
    push_bool_true(&mut s, "allowBlank", v.allow_blank);
    push_bool_false(&mut s, "showInputMessage", v.show_input_message);
    push_bool_false(&mut s, "showErrorMessage", v.show_error_message);
    push_opt(&mut s, "promptTitle", v.prompt_title.as_deref());
    push_opt(&mut s, "prompt", v.prompt.as_deref());
    push_opt(&mut s, "errorTitle", v.error_title.as_deref());
    push_opt(&mut s, "error", v.error.as_deref());
    push_opt(&mut s, "errorStyle", v.error_style.as_deref());
    s + "/>"
}

pub fn conditional_format_line(c: &CondFormatDecl) -> String {
    let mut s = format!(
        "<conditional-format range=\"{}\" type=\"{}\"",
        attr(&c.range),
        attr(&c.ty)
    );
    if c.ty == "dataBar" {
        if let Some(min) = &c.min {
            s.push_str(&format!(" min=\"{}\"", attr(&min.write())));
        }
        if let Some(max) = &c.max {
            s.push_str(&format!(" max=\"{}\"", attr(&max.write())));
        }
        if let Some(fill) = &c.fill {
            s.push_str(&format!(" fill=\"{}\"", attr(&fill.to_string())));
        }
        push_bool_true(&mut s, "stopIfTrue", c.stop_if_true);
        return s + "/>";
    }
    push_opt(&mut s, "operator", c.operator.as_deref());
    push_opt(&mut s, "formula", c.formula.as_deref());
    push_opt(&mut s, "formula2", c.formula2.as_deref());
    if !c.dxf.is_empty() {
        s.push_str(&format!(" dxf=\"{}\"", attr(&c.dxf.write())));
    }
    push_bool_true(&mut s, "stopIfTrue", c.stop_if_true);
    s + "/>"
}

/// An attribute that may hold newlines (`&#10;`, `&#13;`).
fn attr_text(s: &str) -> String {
    attr(s).replace('\n', "&#10;").replace('\r', "&#13;")
}

/// What a row window shows: a table's rows, or a range of a sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WindowOf {
    /// `rows`: the sheet rows shown when not every data row is (`2:101`).
    Table {
        name: String,
        rows: Option<(u32, u32)>,
    },
    /// A range of a sheet (`A1:F50`); its columns are named by their letters.
    Range { sheet: String, range: String },
}

/// The row window (§5.4, round 4 read view A): `<data …>`, the header row
/// `| row | … |`, a delimiter, one line per sheet row (blank rows included),
/// `</data>`. `rows` are `(sheet row, displayed values)`.
pub fn window_text(of: &WindowOf, header: &[String], rows: &[(u32, Vec<String>)]) -> String {
    let open = match of {
        WindowOf::Table { name, rows: None } => format!("<data table=\"{}\">", attr(name)),
        WindowOf::Table {
            name,
            rows: Some((a, b)),
        } => format!("<data table=\"{}\" rows=\"{a}:{b}\">", attr(name)),
        WindowOf::Range { sheet, range } => {
            format!("<data sheet=\"{}\" range=\"{range}\">", attr(sheet))
        }
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

    const TEXT: &str = "---\ntype: spreadsheet\nformat: xlsx\nschema: 1\n---\n\n<sheet name=\"매출\" range=\"A1:E1201\">\n\n<table name=\"Sales\" range=\"A1:E1201\">\n| column | type | format | formula |\n|---|---|---|---|\n| 월 | date | yyyy-mm |  |\n| 매출 | number | #,##0 |  |\n| 이익 | number | #,##0 | =[@매출]-[@원가] |\n</table>\n\n<chart type=\"bar\" data=\"Sales[월],Sales[이익]\" title=\"월별 이익\"/>\n\n<merge range=\"G1:H1\"/>\n\n<validation range=\"B2:B1201\" type=\"list\" formula1=\"&quot;강남,서초,송파&quot;\" allowBlank=\"true\"/>\n\n<conditional-format range=\"F4:F48\" type=\"cellIs\" operator=\"lessThan\" formula=\"3000000\" dxf=\"fill=#FFC7CE\"/>\n\n<keep id=\"k1\" kind=\"picture\" summary=\"a drawing\"/>\n</sheet>\n";

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
        let k1 = Keep {
            id: "k1".into(),
            kind: "picture".into(),
            summary: "a drawing".into(),
        };
        parse_spreadsheet(
            text,
            &Names {
                keeps: Some(vec![k1]),
                ..Default::default()
            },
        )
        .err()
        .unwrap_or_default()
        .into_iter()
        .map(|d| d.to_string())
        .collect()
    }

    #[test]
    fn errors_for_the_model() {
        let e = errs(&TEXT.replace("| 월 | date |", "| 월 | month |"));
        assert!(
            e[0].contains("line 12") && e[0].contains("\"text\", \"number\", \"date\", \"mixed\""),
            "{e:?}"
        );
        let e = errs(&TEXT.replace("</table>\n", ""));
        assert!(e[0].contains("not closed by a line </table>"), "{e:?}");
        let e = errs(&TEXT.replace(" title=", " colour=\"red\" title="));
        assert!(e[0].contains("no attribute \"colour\""), "{e:?}");
        let e = errs(&TEXT.replace("id=\"k1\"", "id=\"k2\""));
        assert!(
            e[0].contains("placeholder id=\"k2\" is not in this file"),
            "{e:?}"
        );
        let e = errs(&TEXT.replace("a drawing", "other"));
        assert!(e[0].contains("was altered"), "{e:?}");
        let e = errs(&TEXT.replace("name=\"Sales\"", "name=\"A1\""));
        assert!(e[0].contains("reads as a cell address"), "{e:?}");
        let e = errs(&TEXT.replace(
            "| 이익 | number | #,##0 | =[@매출]-[@원가] |",
            "| 이익 | number | #,##0 | [@매출] |",
        ));
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
            &WindowOf::Table {
                name: "Sales".into(),
                rows: None,
            },
            &["월".into(), "매출".into()],
            &[
                (2, vec!["2026-01".into(), "12,000,000".into()]),
                (3, vec![String::new(), String::new()]),
            ],
        );
        assert_eq!(w, "<data table=\"Sales\">\n| row | 월 | 매출 |\n|---|---|---|\n| 2 | 2026-01 | 12,000,000 |\n| 3 |  |  |\n</data>\n");
        assert_eq!(split_pipe_row("| a\\|b | c |").unwrap(), ["a|b", "c"]);
    }

    #[test]
    fn merge_lines_round_trip() {
        let s = parse_spreadsheet(TEXT, &Names::default()).unwrap();
        assert_eq!(
            s.sheets[0]
                .items
                .iter()
                .filter_map(|i| match i {
                    SheetItem::Merge(m) => Some(m.range.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            ["G1:H1"]
        );
        let two = TEXT.replace(
            "<merge range=\"G1:H1\"/>",
            "<merge range=\"G1:H1\"/>\n<merge range=\"A2:C2\"/>",
        );
        let s = parse_spreadsheet(&two, &Names::default()).unwrap();
        let canon = serialize_spreadsheet(&s);
        assert_eq!(canon, two);
        assert_eq!(parse_spreadsheet(&canon, &Names::default()).unwrap(), s);
        // Either corner first prints top-left:bottom-right.
        let flipped = TEXT.replace("G1:H1", "H1:G1");
        let s = parse_spreadsheet(&flipped, &Names::default()).unwrap();
        assert_eq!(serialize_spreadsheet(&s), TEXT);
    }

    #[test]
    fn merge_errors() {
        let overlap = TEXT.replace(
            "<merge range=\"G1:H1\"/>",
            "<merge range=\"G1:H1\"/>\n<merge range=\"H1:I2\"/>",
        );
        let e = errs(&overlap);
        assert!(
            e[0].contains("overlaps G1:H1") && e[0].contains("do not overlap"),
            "{e:?}"
        );
        let twice = TEXT.replace(
            "<merge range=\"G1:H1\"/>",
            "<merge range=\"G1:H1\"/>\n<merge range=\"G1:H1\"/>",
        );
        let e = errs(&twice);
        assert!(e[0].contains("overlaps"), "{e:?}");
        let e = errs(&TEXT.replace("G1:H1", "A1"));
        assert!(e[0].contains("one cell"), "{e:?}");
        let e = errs(&TEXT.replace("G1:H1", "A1048577:B1048577"));
        assert!(
            e[0].contains("A1048577:B1048577") && e[0].contains("XFD1048576"),
            "{e:?}"
        );
        let e = errs(&TEXT.replace("G1:H1", "XFE1:XFE2"));
        assert!(e[0].contains("XFE1:XFE2"), "{e:?}");
        let e = errs(&TEXT.replace("G1:H1", "not-a-range"));
        assert!(e[0].contains("not-a-range"), "{e:?}");
    }

    #[test]
    fn hyperlink_and_note_lines_round_trip() {
        let with = TEXT.replace(
            "<merge range=\"G1:H1\"/>",
            "<merge range=\"G1:H1\"/>\n\n<hyperlink range=\"H4\" target=\"https://example.com/sales\"/>\n<hyperlink range=\"A3:A5\" target=\"http://www.apache.org/\" display=\"Apache\" tooltip=\"go\"/>\n\n<note range=\"B2\" author=\"John\" text=\"Some text\"/>\n<note range=\"C5\" author=\"Nick Burch\" text=\"Nick Burch:&#10;This is a comment\"/>",
        );
        let s = parse_spreadsheet(&with, &Names::default()).unwrap();
        let links: Vec<_> = s.sheets[0]
            .items
            .iter()
            .filter_map(|i| match i {
                SheetItem::Hyperlink(h) => Some((
                    h.range.as_str(),
                    h.target.as_deref(),
                    h.display.as_deref(),
                    h.tooltip.as_deref(),
                )),
                _ => None,
            })
            .collect();
        assert_eq!(
            links,
            [
                ("H4", Some("https://example.com/sales"), None, None),
                (
                    "A3:A5",
                    Some("http://www.apache.org/"),
                    Some("Apache"),
                    Some("go")
                ),
            ]
        );
        let notes: Vec<_> = s.sheets[0]
            .items
            .iter()
            .filter_map(|i| match i {
                SheetItem::Note(n) => Some((n.range.as_str(), n.author.as_str(), n.text.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(
            notes,
            [
                ("B2", "John", "Some text"),
                ("C5", "Nick Burch", "Nick Burch:\nThis is a comment"),
            ]
        );
        let canon = serialize_spreadsheet(&s);
        assert_eq!(canon, with);
        assert_eq!(parse_spreadsheet(&canon, &Names::default()).unwrap(), s);
        let loc = TEXT.replace(
            "<merge range=\"G1:H1\"/>",
            "<hyperlink range=\"B2\" location=\"Sheet2!A1\"/>",
        );
        let s = parse_spreadsheet(&loc, &Names::default()).unwrap();
        let h = s.sheets[0]
            .items
            .iter()
            .find_map(|i| match i {
                SheetItem::Hyperlink(h) => Some(h),
                _ => None,
            })
            .unwrap();
        assert_eq!(h.range, "B2");
        assert_eq!(h.location.as_deref(), Some("Sheet2!A1"));
        assert!(h.target.is_none());
        assert_eq!(serialize_spreadsheet(&s), loc);
    }

    #[test]
    fn shape_and_group_lines_round_trip() {
        let with = TEXT.replace(
            "<merge range=\"G1:H1\"/>",
            "<shape name=\"입력 안내\" anchor=\"B25:D27\">아래에 입력하면 표가 늘어납니다.</shape>\n<shape name=\"빈\" anchor=\"A1\"/>\n<group name=\"신호등\">\n<shape name=\"매출 배지\" anchor=\"F2:G3\">매출 +12%</shape>\n<shape name=\"이익 배지\" anchor=\"F4:G5\">이익 +8%</shape>\n</group>",
        );
        let s = parse_spreadsheet(&with, &Names::default()).unwrap();
        let items: Vec<_> = s.sheets[0]
            .items
            .iter()
            .filter_map(|i| match i {
                SheetItem::Shape(sh) => {
                    Some(format!("shape {} {} {:?}", sh.name, sh.anchor, sh.text))
                }
                SheetItem::Group(g) => Some(format!("group {} {}", g.name, g.items.len())),
                _ => None,
            })
            .collect();
        assert_eq!(
            items,
            [
                "shape 입력 안내 B25:D27 \"아래에 입력하면 표가 늘어납니다.\"",
                "shape 빈 A1 \"\"",
                "group 신호등 2",
            ]
        );
        let DrawingItem::Shape(inner) = &s.sheets[0]
            .items
            .iter()
            .find_map(|i| match i {
                SheetItem::Group(g) => Some(&g.items[0]),
                _ => None,
            })
            .unwrap()
        else {
            panic!("grouped shape");
        };
        assert_eq!(inner.name, "매출 배지");
        assert_eq!(inner.anchor, "F2:G3");
        let nl = TEXT.replace(
            "<merge range=\"G1:H1\"/>",
            "<shape name=\"Tip\" anchor=\"B2\">line1&#10;line2</shape>",
        );
        let s = parse_spreadsheet(&nl, &Names::default()).unwrap();
        let t = s.sheets[0]
            .items
            .iter()
            .find_map(|i| match i {
                SheetItem::Shape(sh) => Some(sh.text.as_str()),
                _ => None,
            })
            .unwrap();
        assert_eq!(t, "line1\nline2");
        assert_eq!(serialize_spreadsheet(&s), nl);
        let canon = serialize_spreadsheet(&parse_spreadsheet(&with, &Names::default()).unwrap());
        assert_eq!(canon, with);
    }

    #[test]
    fn shape_and_group_errors() {
        let e = errs(&TEXT.replace("<merge range=\"G1:H1\"/>", "<shape anchor=\"B2\"/>"));
        assert!(e[0].contains("name and an anchor"), "{e:?}");
        let e = errs(&TEXT.replace(
            "<merge range=\"G1:H1\"/>",
            "<shape name=\"Tip\" anchor=\"not-a-range\"/>",
        ));
        assert!(e[0].contains("anchor="), "{e:?}");
        let e = errs(&TEXT.replace(
            "<merge range=\"G1:H1\"/>",
            "<shape name=\"Tip\" anchor=\"B2\" fill=\"red\"/>",
        ));
        assert!(e[0].contains("no attribute \"fill\""), "{e:?}");
        let e = errs(&TEXT.replace("<merge range=\"G1:H1\"/>", "<group name=\"g\">\n</group>"));
        assert!(e[0].contains("empty"), "{e:?}");
        let e = errs(&TEXT.replace("<merge range=\"G1:H1\"/>", "<group name=\"g\">"));
        assert!(e[0].contains("not closed"), "{e:?}");
        let e = errs(&TEXT.replace("<merge range=\"G1:H1\"/>", "</group>"));
        assert!(e[0].contains("closes no group"), "{e:?}");
    }

    #[test]
    fn hyperlink_and_note_errors() {
        let e = errs(&TEXT.replace("<merge range=\"G1:H1\"/>", "<hyperlink range=\"A1\"/>"));
        assert!(e[0].contains("target URL or a location"), "{e:?}");
        let e = errs(&TEXT.replace(
            "<merge range=\"G1:H1\"/>",
            "<hyperlink range=\"A1\" target=\"http://a\"/>\n<hyperlink range=\"A1\" target=\"http://b\"/>",
        ));
        assert!(e[0].contains("overlaps A1"), "{e:?}");
        let e = errs(&TEXT.replace(
            "<merge range=\"G1:H1\"/>",
            "<note range=\"B2\" author=\"John\"/>",
        ));
        assert!(e[0].contains("range, an author and text"), "{e:?}");
        let e = errs(&TEXT.replace(
            "<merge range=\"G1:H1\"/>",
            "<note range=\"B2\" author=\"A\" text=\"one\"/>\n<note range=\"B2\" author=\"B\" text=\"two\"/>",
        ));
        assert!(e[0].contains("overlaps B2"), "{e:?}");
    }

    #[test]
    fn validation_and_conditional_format_lines_round_trip() {
        let s = parse_spreadsheet(TEXT, &Names::default()).unwrap();
        let v = s.sheets[0]
            .items
            .iter()
            .find_map(|i| match i {
                SheetItem::Validation(v) => Some(v),
                _ => None,
            })
            .unwrap();
        assert_eq!(v.range, "B2:B1201");
        assert_eq!(v.ty, "list");
        assert_eq!(v.formula1.as_deref(), Some("\"강남,서초,송파\""));
        assert!(v.allow_blank);
        let c = s.sheets[0]
            .items
            .iter()
            .find_map(|i| match i {
                SheetItem::ConditionalFormat(c) => Some(c),
                _ => None,
            })
            .unwrap();
        assert_eq!(c.range, "F4:F48");
        assert_eq!(c.ty, "cellIs");
        assert_eq!(c.operator.as_deref(), Some("lessThan"));
        assert_eq!(c.formula.as_deref(), Some("3000000"));
        assert_eq!(c.dxf.to_string(), "fill=#FFC7CE");
        assert_eq!(serialize_spreadsheet(&s), TEXT);
        let extra = TEXT.replace(
            "<validation range=\"B2:B1201\" type=\"list\" formula1=\"&quot;강남,서초,송파&quot;\" allowBlank=\"true\"/>",
            "<validation range=\"C2:C10\" type=\"whole\" operator=\"greaterThan\" formula1=\"1\" allowBlank=\"true\" showErrorMessage=\"false\" promptTitle=\"Integer Input\" errorStyle=\"warning\"/>",
        );
        let s = parse_spreadsheet(&extra, &Names::default()).unwrap();
        let v = s.sheets[0]
            .items
            .iter()
            .find_map(|i| match i {
                SheetItem::Validation(v) => Some(v),
                _ => None,
            })
            .unwrap();
        assert_eq!(v.ty, "whole");
        assert_eq!(v.operator.as_deref(), Some("greaterThan"));
        assert_eq!(v.show_error_message, Some(false));
        assert_eq!(v.error_style.as_deref(), Some("warning"));
        assert_eq!(serialize_spreadsheet(&s), extra);
        let expr = TEXT.replace(
            "<conditional-format range=\"F4:F48\" type=\"cellIs\" operator=\"lessThan\" formula=\"3000000\" dxf=\"fill=#FFC7CE\"/>",
            "<conditional-format range=\"B9\" type=\"expression\" formula=\"$A$8>5\" dxf=\"color=#FF0000 bold\" stopIfTrue=\"true\"/>",
        );
        let s = parse_spreadsheet(&expr, &Names::default()).unwrap();
        let c = s.sheets[0]
            .items
            .iter()
            .find_map(|i| match i {
                SheetItem::ConditionalFormat(c) => Some(c),
                _ => None,
            })
            .unwrap();
        assert_eq!(c.ty, "expression");
        assert!(c.operator.is_none());
        assert!(c.stop_if_true);
        assert_eq!(serialize_spreadsheet(&s), expr);
        let bar = TEXT.replace(
            "<conditional-format range=\"F4:F48\" type=\"cellIs\" operator=\"lessThan\" formula=\"3000000\" dxf=\"fill=#FFC7CE\"/>",
            "<conditional-format range=\"E4:E15\" type=\"dataBar\" min=\"num:0\" max=\"num:월간수입합계\" fill=\"#638EC6\"/>",
        );
        let s = parse_spreadsheet(&bar, &Names::default()).unwrap();
        let c = s.sheets[0]
            .items
            .iter()
            .find_map(|i| match i {
                SheetItem::ConditionalFormat(c) => Some(c),
                _ => None,
            })
            .unwrap();
        assert_eq!(c.ty, "dataBar");
        assert_eq!(c.min.as_ref().map(Cfvo::write).as_deref(), Some("num:0"));
        assert_eq!(
            c.max.as_ref().map(Cfvo::write).as_deref(),
            Some("num:월간수입합계")
        );
        assert_eq!(
            c.fill.as_ref().map(|c| c.to_string()).as_deref(),
            Some("#638EC6")
        );
        assert_eq!(serialize_spreadsheet(&s), bar);
        let auto = TEXT.replace(
            "<conditional-format range=\"F4:F48\" type=\"cellIs\" operator=\"lessThan\" formula=\"3000000\" dxf=\"fill=#FFC7CE\"/>",
            "<conditional-format range=\"G4:G15\" type=\"dataBar\" min=\"min\" max=\"max\" fill=\"accent1\"/>",
        );
        let s = parse_spreadsheet(&auto, &Names::default()).unwrap();
        assert_eq!(serialize_spreadsheet(&s), auto);
        let formula = TEXT.replace(
            "<conditional-format range=\"F4:F48\" type=\"cellIs\" operator=\"lessThan\" formula=\"3000000\" dxf=\"fill=#FFC7CE\"/>",
            "<conditional-format range=\"A1\" type=\"dataBar\" min=\"formula:A1\" max=\"percent:100\" fill=\"#FFC7CE\"/>",
        );
        let s = parse_spreadsheet(&formula, &Names::default()).unwrap();
        let c = s.sheets[0]
            .items
            .iter()
            .find_map(|i| match i {
                SheetItem::ConditionalFormat(c) => Some(c),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            c.min.as_ref().map(Cfvo::write).as_deref(),
            Some("formula:=A1")
        );
        assert_eq!(
            serialize_spreadsheet(&s),
            formula.replace("min=\"formula:A1\"", "min=\"formula:=A1\"")
        );
    }

    #[test]
    fn list_formula1_is_excel_formula_text() {
        let quoted = TEXT.replace(
            "formula1=\"&quot;강남,서초,송파&quot;\"",
            "formula1=\"&quot;Yes&quot;\"",
        );
        let s = parse_spreadsheet(&quoted, &Names::default()).unwrap();
        let v = s.sheets[0]
            .items
            .iter()
            .find_map(|i| match i {
                SheetItem::Validation(v) => Some(v),
                _ => None,
            })
            .unwrap();
        assert_eq!(v.formula1.as_deref(), Some("\"Yes\""));
        assert_eq!(serialize_spreadsheet(&s), quoted);
        let concat = TEXT.replace(
            "formula1=\"&quot;강남,서초,송파&quot;\"",
            "formula1=\"&quot;A&quot;&amp;&quot;B&quot;\"",
        );
        let s = parse_spreadsheet(&concat, &Names::default()).unwrap();
        let v = s.sheets[0]
            .items
            .iter()
            .find_map(|i| match i {
                SheetItem::Validation(v) => Some(v),
                _ => None,
            })
            .unwrap();
        assert_eq!(v.formula1.as_deref(), Some("\"A\"&\"B\""));
        assert_eq!(serialize_spreadsheet(&s), concat);
        let range = TEXT.replace(
            "formula1=\"&quot;강남,서초,송파&quot;\"",
            "formula1=\"$A$1:$A$5\"",
        );
        let s = parse_spreadsheet(&range, &Names::default()).unwrap();
        let v = s.sheets[0]
            .items
            .iter()
            .find_map(|i| match i {
                SheetItem::Validation(v) => Some(v),
                _ => None,
            })
            .unwrap();
        assert_eq!(v.formula1.as_deref(), Some("$A$1:$A$5"));
        assert_eq!(serialize_spreadsheet(&s), range);
    }

    #[test]
    fn validation_and_conditional_format_errors() {
        let e = errs(&TEXT.replace(
            "<validation range=\"B2:B1201\" type=\"list\" formula1=\"&quot;강남,서초,송파&quot;\" allowBlank=\"true\"/>",
            "<validation range=\"B2\"/>",
        ));
        assert!(e[0].contains("range and a type"), "{e:?}");
        let e = errs(&TEXT.replace("type=\"list\"", "type=\"dropdown\""));
        assert!(e[0].contains("dropdown"), "{e:?}");
        let e = errs(&TEXT.replace(
            "type=\"cellIs\" operator=\"lessThan\"",
            "type=\"colorScale\" operator=\"lessThan\"",
        ));
        assert!(
            e[0].contains("colorScale") && e[0].contains("placeholder"),
            "{e:?}"
        );
        let e = errs(&TEXT.replace(
            "type=\"cellIs\" operator=\"lessThan\" formula=\"3000000\" dxf=\"fill=#FFC7CE\"",
            "type=\"dataBar\" min=\"num:0\" max=\"max\"",
        ));
        assert!(e[0].contains("min, max and fill"), "{e:?}");
        let e = errs(&TEXT.replace(
            "type=\"cellIs\" operator=\"lessThan\"",
            "type=\"dataBar\" operator=\"lessThan\"",
        ));
        assert!(
            e[0].contains("dataBar") && e[0].contains("not operator"),
            "{e:?}"
        );
        let e = errs(&TEXT.replace(
            "<conditional-format range=\"F4:F48\" type=\"cellIs\" operator=\"lessThan\" formula=\"3000000\" dxf=\"fill=#FFC7CE\"/>",
            "<conditional-format range=\"E4\" type=\"dataBar\" min=\"min:0\" max=\"max\" fill=\"#638EC6\"/>",
        ));
        assert!(e[0].contains("min") && e[0].contains("no value"), "{e:?}");
        let e = errs(&TEXT.replace(
            "<conditional-format range=\"F4:F48\" type=\"cellIs\" operator=\"lessThan\" formula=\"3000000\" dxf=\"fill=#FFC7CE\"/>",
            "<conditional-format range=\"E4\" type=\"dataBar\" min=\"percent:120\" max=\"max\" fill=\"#638EC6\"/>",
        ));
        assert!(e[0].contains("0–100"), "{e:?}");
        let e = errs(&TEXT.replace(
            "type=\"cellIs\" operator=\"lessThan\" formula=\"3000000\" dxf=\"fill=#FFC7CE\"",
            "type=\"cellIs\" operator=\"lessThan\" formula=\"3000000\" dxf=\"fill=#FFC7CE\" min=\"min\"",
        ));
        assert!(e[0].contains("dataBar"), "{e:?}");
        let e = errs(&TEXT.replace(
            "type=\"cellIs\" operator=\"lessThan\"",
            "type=\"expression\" operator=\"lessThan\"",
        ));
        assert!(e[0].contains("no operator"), "{e:?}");
        let e = errs(&TEXT.replace("operator=\"lessThan\"", "operator=\"bigger\""));
        assert!(e[0].contains("bigger"), "{e:?}");
    }
}
