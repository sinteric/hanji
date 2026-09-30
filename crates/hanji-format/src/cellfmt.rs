//! A workbook's cell formatting (DESIGN.md §5.4, round 6 part D): the
//! vocabulary's cell keys (§5.1) as one value per key, the `<format …/>`
//! lines that show it in a sheet block, and the `set` of a `format` range
//! operation that writes it.
//!
//! - `<format default font=Calibri size=11pt color=tx1/>` after the front
//!   matter: the Normal style's formatting. A key it leaves out is none,
//!   off, general alignment, bottom, or indent 0.
//! - `<format range="B4:C7" fill=#D9D9D9 bold/>` in a sheet: every cell of
//!   the rectangle has what the line writes, beyond the default; a key the
//!   line leaves out is the default's. A cell is in at most one line.
//!
//! The lines are read-only in the text: formatting is written by the
//! `format` operation.

use std::fmt;

use crate::vocab::{self, Border, Color, Fill};

/// Horizontal alignment in a cell: the vocabulary's, and `general` (Excel's
/// default: text left, numbers right), which a line never writes.
pub const ALIGN: &[&str] = &["general", "left", "center", "right", "justify", "distribute"];
/// The keys of a cell, in canonical order.
pub const KEYS: &[&str] = &[
    "fill",
    "border-top",
    "border-right",
    "border-bottom",
    "border-left",
    "valign",
    "align",
    "font",
    "size",
    "color",
    "bold",
    "italic",
    "underline",
    "strike",
    "indent",
];

/// A cell's formatting, one optional value per key: on a line, what differs
/// from the default; in an operation, what it sets; for a cell, every key.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellFormat {
    pub fill: Option<Fill>,
    /// Top, right, bottom, left.
    pub borders: [Option<Border>; 4],
    pub valign: Option<String>,
    pub align: Option<String>,
    pub font: Option<String>,
    /// Hundredths of a point.
    pub size: Option<i64>,
    pub color: Option<Color>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strike: Option<bool>,
    /// Excel's indent, in levels.
    pub indent: Option<u32>,
}

pub const SIDES: [&str; 4] = ["border-top", "border-right", "border-bottom", "border-left"];

impl CellFormat {
    pub fn is_empty(&self) -> bool {
        *self == CellFormat::default()
    }

    /// What a cell has when nothing sets it, beyond Normal's font, size and colour.
    pub fn implicit() -> CellFormat {
        CellFormat {
            fill: Some(Fill::None),
            borders: [Some(Border::None), Some(Border::None), Some(Border::None), Some(Border::None)],
            valign: Some("bottom".into()),
            align: Some("general".into()),
            bold: Some(false),
            italic: Some(false),
            underline: Some(false),
            strike: Some(false),
            indent: Some(0),
            ..Default::default()
        }
    }

    /// `self` with every value `over` has.
    pub fn overlay(&self, over: &CellFormat) -> CellFormat {
        fn pick<T: Clone>(a: &Option<T>, b: &Option<T>) -> Option<T> {
            b.clone().or_else(|| a.clone())
        }
        CellFormat {
            fill: pick(&self.fill, &over.fill),
            borders: [0, 1, 2, 3].map(|k| pick(&self.borders[k], &over.borders[k])),
            valign: pick(&self.valign, &over.valign),
            align: pick(&self.align, &over.align),
            font: pick(&self.font, &over.font),
            size: pick(&self.size, &over.size),
            color: pick(&self.color, &over.color),
            bold: pick(&self.bold, &over.bold),
            italic: pick(&self.italic, &over.italic),
            underline: pick(&self.underline, &over.underline),
            strike: pick(&self.strike, &over.strike),
            indent: pick(&self.indent, &over.indent),
        }
    }

    /// The values of `self` that differ from `base`'s.
    pub fn diff(&self, base: &CellFormat) -> CellFormat {
        fn keep<T: Clone + PartialEq>(a: &Option<T>, b: &Option<T>) -> Option<T> {
            if a == b {
                None
            } else {
                a.clone()
            }
        }
        CellFormat {
            fill: keep(&self.fill, &base.fill),
            borders: [0, 1, 2, 3].map(|k| keep(&self.borders[k], &base.borders[k])),
            valign: keep(&self.valign, &base.valign),
            align: keep(&self.align, &base.align),
            font: keep(&self.font, &base.font),
            size: keep(&self.size, &base.size),
            color: keep(&self.color, &base.color),
            bold: keep(&self.bold, &base.bold),
            italic: keep(&self.italic, &base.italic),
            underline: keep(&self.underline, &base.underline),
            strike: keep(&self.strike, &base.strike),
            indent: keep(&self.indent, &base.indent),
        }
    }

    /// Sets key `key` from its text value (`None`: a bare flag).
    pub fn set(&mut self, key: &str, v: Option<&str>) -> Result<(), String> {
        fn need<'a>(key: &str, v: Option<&'a str>) -> Result<&'a str, String> {
            v.ok_or_else(|| format!("{key} needs a value: {key}=…"))
        }
        match key {
            "fill" => self.fill = Some(Fill::parse(need(key, v)?)?),
            "border" => {
                let b = Border::parse(need(key, v)?)?;
                self.borders = [0, 1, 2, 3].map(|_| Some(b.clone()));
            }
            k if SIDES.contains(&k) => {
                let at = SIDES.iter().position(|s| *s == k).unwrap();
                self.borders[at] = Some(Border::parse(need(key, v)?)?);
            }
            "valign" => self.valign = Some(vocab::parse_choice(key, need(key, v)?, vocab::VALIGN)?),
            "align" => self.align = Some(vocab::parse_choice(key, need(key, v)?, ALIGN)?),
            "font" => {
                let f = need(key, v)?.trim();
                if f.is_empty() {
                    return Err("font= names a font, such as font=Calibri".into());
                }
                self.font = Some(f.to_string());
            }
            "size" => {
                let s = vocab::parse_length(need(key, v)?, false)?;
                if s == 0 {
                    return Err("size is above 0pt".into());
                }
                self.size = Some(s);
            }
            "color" => self.color = Some(Color::parse(need(key, v)?)?),
            "bold" => self.bold = Some(vocab::parse_flag(key, v)?),
            "italic" => self.italic = Some(vocab::parse_flag(key, v)?),
            "underline" => self.underline = Some(vocab::parse_flag(key, v)?),
            "strike" => self.strike = Some(vocab::parse_flag(key, v)?),
            "indent" => {
                let s = need(key, v)?;
                self.indent = Some(s.parse().map_err(|_| {
                    format!("indent={s}: indent is a whole number of indent levels (indent=1), not a length")
                })?);
            }
            _ => return Err(unknown(key)),
        }
        Ok(())
    }

    /// The `key=value` list canonical form writes: vocabulary order, four
    /// equal sides as `border`, flags bare when on.
    pub fn write(&self) -> String {
        let mut out: Vec<String> = vec![];
        let kv = |k: &str, v: String| format!("{k}={}", vocab::quote(&v));
        if let Some(f) = &self.fill {
            out.push(kv("fill", f.to_string()));
        }
        let all = self.borders.iter().all(|b| b.is_some() && *b == self.borders[0]);
        if all {
            out.push(kv("border", self.borders[0].as_ref().unwrap().to_string()));
        } else {
            for (k, b) in self.borders.iter().enumerate() {
                if let Some(b) = b {
                    out.push(kv(SIDES[k], b.to_string()));
                }
            }
        }
        if let Some(v) = &self.valign {
            out.push(kv("valign", v.clone()));
        }
        if let Some(v) = &self.align {
            out.push(kv("align", v.clone()));
        }
        if let Some(v) = &self.font {
            out.push(kv("font", v.clone()));
        }
        if let Some(v) = self.size {
            out.push(kv("size", vocab::length_text(v)));
        }
        if let Some(v) = &self.color {
            out.push(kv("color", v.to_string()));
        }
        for (k, f) in
            [("bold", self.bold), ("italic", self.italic), ("underline", self.underline), ("strike", self.strike)]
        {
            match f {
                Some(true) => out.push(k.to_string()),
                Some(false) => out.push(format!("{k}=no")),
                None => {}
            }
        }
        if let Some(v) = self.indent {
            out.push(kv("indent", v.to_string()));
        }
        out.join(" ")
    }

    /// Reads a `key=value` list (a line's attributes).
    pub fn parse(src: &str) -> Result<CellFormat, (usize, String)> {
        let mut f = CellFormat::default();
        for (k, v, col) in vocab::attrs(src)? {
            f.set(&k, v.as_deref()).map_err(|m| (col, m))?;
        }
        Ok(f)
    }
}

impl fmt::Display for CellFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.write())
    }
}

/// The refusal for a key a cell does not have, with a hint where one helps.
pub fn unknown(key: &str) -> String {
    let hint = match key {
        "style" => " A cell's named style is shown (style=\"Name\") but not written by an operation yet.".to_string(),
        k if k.contains(':') || k == "background" || k == "background-color" || k == "font-size" => {
            format!(" {}", vocab::css_refusal())
        }
        "outline" => " outline is an operation's key: it sets the outer edges of the range.".to_string(),
        "indent-left" | "first-line" | "line-spacing" | "space-before" | "space-after" => {
            " A cell's indent is indent=N, in levels.".to_string()
        }
        _ => String::new(),
    };
    format!("{key} is not a cell's formatting key; a cell has {}.{hint}", KEYS.join(", "))
}

/// A `<format …/>` line of a sheet: a rectangle of cells and what they
/// have beyond the default (and the named cell style they are in, when it
/// is not Normal).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatLine {
    pub range: String,
    pub style: Option<String>,
    pub format: CellFormat,
}

impl FormatLine {
    pub fn line(&self) -> String {
        let mut s = format!("<format range=\"{}\"", self.range);
        if let Some(st) = &self.style {
            s.push_str(&format!(" style={}", vocab::quote(st)));
        }
        let body = self.format.write();
        if !body.is_empty() {
            s.push(' ');
            s.push_str(&body);
        }
        s + "/>"
    }
}

/// The default line: Normal's formatting.
pub fn default_line(f: &CellFormat) -> String {
    let body = f.write();
    if body.is_empty() {
        "<format default/>".into()
    } else {
        format!("<format default {body}/>")
    }
}

/// What a `<format …/>` line holds: the default, or a range line.
#[derive(Debug)]
pub enum Parsed {
    Default(CellFormat),
    Range(FormatLine),
}

pub const FORM: &str =
    "<format range=\"B4:C7\" key=value …/> (or <format default key=value …/> after the front matter)";

/// Reads a `<format …/>` line (trimmed). The error is at a column of the line.
pub fn parse_line(t: &str) -> Result<Parsed, (usize, String)> {
    let inner = t
        .strip_prefix("<format")
        .and_then(|r| r.strip_suffix("/>"))
        .filter(|r| r.is_empty() || r.starts_with(char::is_whitespace))
        .ok_or((1, format!("a format line is {FORM}")))?;
    let attrs = vocab::attrs(inner).map_err(|(c, m)| (c + 7, m))?;
    let mut range = None;
    let mut style = None;
    let mut default = false;
    let mut f = CellFormat::default();
    for (k, v, col) in attrs {
        let col = col + 7;
        match (k.as_str(), v) {
            ("default", None) => default = true,
            ("range", Some(r)) => range = Some(r),
            ("style", Some(s)) => style = Some(s),
            (k, v) => f.set(k, v.as_deref()).map_err(|m| (col, m))?,
        }
    }
    match (default, range) {
        (true, None) if style.is_none() => Ok(Parsed::Default(f)),
        (false, Some(range)) => Ok(Parsed::Range(FormatLine { range, style, format: f })),
        _ => Err((1, format!("a format line is {FORM}"))),
    }
}

/// A grid of cells with their formatting → the rectangles canonical form
/// writes: runs of equal cells along each row, then equal runs stacked in
/// consecutive rows. `cells` is `(row, col, value)`, any order; the result
/// is `(r1, c1, r2, c2, value)` sorted by top-left cell.
pub fn rectangles<T: Clone + PartialEq>(cells: &[(u32, u32, T)]) -> Vec<(u32, u32, u32, u32, T)> {
    let mut sorted: Vec<&(u32, u32, T)> = cells.iter().collect();
    sorted.sort_by_key(|c| (c.0, c.1));
    // Runs along a row.
    let mut runs: Vec<(u32, u32, u32, T)> = vec![];
    for (r, c, v) in sorted {
        match runs.last_mut() {
            Some(last) if last.0 == *r && last.2 + 1 == *c && last.3 == *v => last.2 = *c,
            _ => runs.push((*r, *c, *c, v.clone())),
        }
    }
    // Stack a run under an open rectangle with the same columns and value
    // that ended on the row above.
    let mut done: Vec<(u32, u32, u32, u32, T)> = vec![];
    let mut open: Vec<(u32, u32, u32, u32, T)> = vec![];
    for (r, c1, c2, v) in runs {
        // Close what cannot grow any more.
        let mut k = 0;
        while k < open.len() {
            if open[k].2 + 1 < r {
                done.push(open.remove(k));
            } else {
                k += 1;
            }
        }
        match open.iter_mut().find(|o| o.1 == c1 && o.3 == c2 && o.4 == v && o.2 + 1 == r) {
            Some(o) => o.2 = r,
            None => open.push((r, c1, r, c2, v)),
        }
    }
    done.extend(open);
    done.sort_by_key(|x| (x.0, x.1));
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_reads_and_writes_in_canonical_order() {
        let Parsed::Range(l) =
            parse_line("<format range=\"A1:B2\" bold fill=#D9D9D9 border=\"0.75pt solid #000000\" indent=1/>").unwrap()
        else {
            panic!()
        };
        assert_eq!(l.line(), "<format range=\"A1:B2\" fill=#D9D9D9 border=\"0.75pt solid #000000\" bold indent=1/>");
        let Parsed::Default(d) = parse_line("<format default font=Calibri size=11pt color=tx1/>").unwrap() else {
            panic!()
        };
        assert_eq!(default_line(&d), "<format default font=Calibri size=11pt color=tx1/>");
        assert!(parse_line("<format range=\"A1\" indent=10pt/>").unwrap_err().1.contains("indent levels"));
        assert!(parse_line("<format range=\"A1\" first-line=10pt/>").unwrap_err().1.contains("indent=N"));
    }

    #[test]
    fn a_format_operation_reads_its_set() {
        use crate::ops::{parse_ops, RangeOp};
        let ops = parse_ops(r##"[{"op": "format", "range": "S!A1:D1", "set": {"fill": "accent2+80%", "bold": true, "italic": false, "size": 14, "indent": 2, "border": "0.75pt solid #000000", "outline": "2pt solid #1F3864"}}]"##).unwrap();
        let RangeOp::Format { range, set, outline } = &ops[0] else { panic!() };
        assert_eq!(range, "S!A1:D1");
        assert_eq!(set.write(), "fill=accent2+80% border=\"0.75pt solid #000000\" size=14pt bold italic=no indent=2");
        assert_eq!(outline.as_ref().unwrap().to_string(), "2pt solid #1F3864");
        let e = parse_ops(r##"[{"op": "format", "range": "S!A1", "set": {"bold": "maybe"}}]"##).unwrap_err();
        assert!(e[0].to_string().contains("true or false"), "{}", e[0]);
    }

    #[test]
    fn equal_cells_become_rectangles() {
        let cells = vec![(1, 1, 'a'), (1, 2, 'a'), (2, 1, 'a'), (2, 2, 'a'), (2, 3, 'b'), (4, 1, 'a'), (4, 2, 'a')];
        assert_eq!(rectangles(&cells), vec![(1, 1, 2, 2, 'a'), (2, 3, 2, 3, 'b'), (4, 1, 4, 2, 'a')]);
    }
}
