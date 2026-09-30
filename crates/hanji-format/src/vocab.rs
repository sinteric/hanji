//! The direct-formatting vocabulary (DESIGN.md §5.1, fluency round 6): one
//! small, fixed set of values for all four formats, each parsed from what a
//! text writes and written back in one canonical spelling. Where a value
//! attaches (a tag, a slot marker, `{…}`, `[text]{…}`, a range line) is each
//! kind's own (§5.2–§5.4); this module knows only the values.
//!
//! - lengths: points with `pt`, at most two decimals (`0.34pt`, `20pt`); a
//!   bare number reads as points;
//! - colours: `#RRGGBB`, or a theme colour by its name, kept a name:
//!   `accent1+40%` lighter and `accent1-25%` darker (Office's presets),
//!   `accent1*` another transform (shown, kept while left as written, not
//!   writable), then `/55%` opacity;
//! - borders: `none`, or `"<width>pt <style> <colour>"`;
//! - fills: `none`, a colour, or `gradient`, `pattern`, `picture` (shown,
//!   not writable);
//! - `align`, `valign`, `line-spacing` (`160%`, `14pt`, `"at-least 14pt"`,
//!   `"gap 4pt"`), and flags (`bold` on, `bold=no` off).
//!
//! Values are `key=value` pairs separated by spaces, a value with a space
//! quoted; not CSS (round 1: models do not write CSS).

use std::fmt;

/// Paragraph keys, in canonical order.
pub const PARA_KEYS: &[&str] =
    &["align", "indent-left", "indent-right", "first-line", "space-before", "space-after", "line-spacing"];
/// Text keys, in canonical order.
pub const TEXT_KEYS: &[&str] = &["font", "size", "color", "bold", "italic", "underline", "strike"];
/// Box keys (a cell, a shape, a paragraph's shading), in canonical order.
pub const BOX_KEYS: &[&str] =
    &["fill", "border", "border-top", "border-right", "border-bottom", "border-left", "valign"];
/// Spreadsheet-only keys.
pub const XLSX_KEYS: &[&str] = &["indent"];
/// Flags: bare is on, `=no` is off.
pub const FLAGS: &[&str] = &["bold", "italic", "underline", "strike"];

/// Every key, in the order canonical form writes them.
pub fn order() -> impl Iterator<Item = &'static str> {
    BOX_KEYS.iter().chain(PARA_KEYS).chain(TEXT_KEYS).chain(XLSX_KEYS).copied()
}

/// A key's place in canonical order (`None`: not a key of the vocabulary).
pub fn rank(key: &str) -> Option<usize> {
    order().position(|k| k == key)
}

// ---------------------------------------------------------------- lengths

/// A length in hundredths of a point, as the text writes it: `0.34pt`, `20pt`, `-58.96pt`.
pub fn length_text(h: i64) -> String {
    let (sign, a) = if h < 0 { ("-", -h) } else { ("", h) };
    let (w, f) = (a / 100, a % 100);
    match f {
        0 => format!("{sign}{w}pt"),
        f if f % 10 == 0 => format!("{sign}{w}.{}pt", f / 10),
        f => format!("{sign}{w}.{f:02}pt"),
    }
}

/// `12pt`, `0.34pt`, `-10pt` or a bare number of points → hundredths of a
/// point, rounded; `negative`: whether a negative length is allowed.
pub fn parse_length(v: &str, negative: bool) -> Result<i64, String> {
    let t = v.trim();
    let n = t.strip_suffix("pt").unwrap_or(t).trim_end();
    let u = n.strip_prefix('-').unwrap_or(n);
    let ok = !u.is_empty()
        && u.chars().all(|c| c.is_ascii_digit() || c == '.')
        && u.matches('.').count() <= 1
        && !u.starts_with('.')
        && !u.ends_with('.');
    let x: f64 = n
        .parse()
        .ok()
        .filter(|_| ok)
        .ok_or_else(|| format!("{v:?} is not a length; a length is in points, such as 12pt or 0.5pt"))?;
    if x < 0.0 && !negative {
        return Err(format!("{v:?}: this length is 0pt or more"));
    }
    if x.abs() >= 1e7 {
        return Err(format!("{v:?} is too long a length"));
    }
    Ok((x * 100.0).round() as i64)
}

// ---------------------------------------------------------------- colours

/// Theme colours a colour may name (DrawingML's scheme colours; docx and
/// xlsx theme colours are named the same way), kept as names.
pub const THEME_COLORS: &[&str] = &[
    "tx1", "bg1", "tx2", "bg2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6", "hlink", "folHlink",
    "dk1", "lt1", "dk2", "lt2",
];

/// What a colour may be written as, for messages.
pub const COLOR_FORM: &str = "#RRGGBB, or a theme colour (tx1, bg1, tx2, bg2, accent1 … accent6, hlink, folHlink, dk1, lt1), lighter or darker by a percent (accent1+40%, accent1-25%), with /NN% opacity after it if wanted (#1F4E79/50%)";

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ColorBase {
    Rgb([u8; 3]),
    Theme(String),
}

/// How a theme colour is adjusted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tint {
    #[default]
    None,
    /// Office's "Lighter NN%" (1–99).
    Lighter(u8),
    /// Office's "Darker NN%" (1–99).
    Darker(u8),
    /// Another transform the file stores: shown `*`, kept while left as
    /// written, never written anew.
    Other,
}

/// A colour of the vocabulary.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Color {
    pub base: ColorBase,
    pub tint: Tint,
    /// Opacity in percent, below 100 (`None`: opaque).
    pub alpha: Option<u8>,
}

impl Color {
    pub fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { base: ColorBase::Rgb([r, g, b]), tint: Tint::None, alpha: None }
    }

    pub fn theme(name: &str) -> Color {
        Color { base: ColorBase::Theme(name.to_string()), tint: Tint::None, alpha: None }
    }

    /// Whether a text may write this colour anew (not a `*` one).
    pub fn is_writable(&self) -> bool {
        self.tint != Tint::Other
    }

    /// A colour as a text writes it.
    pub fn parse(v: &str) -> Result<Color, String> {
        let err = || format!("{v:?} is not a colour; a colour is {COLOR_FORM}");
        let s = v.trim();
        let (body, alpha) = match s.rsplit_once('/') {
            Some((b, a)) => {
                let n: u32 = a
                    .strip_suffix('%')
                    .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                    .and_then(|n| n.parse().ok())
                    .ok_or_else(err)?;
                if n > 100 {
                    return Err(format!("{v:?}: an opacity is 0% to 100%"));
                }
                (b, (n < 100).then_some(n as u8))
            }
            None => (s, None),
        };
        if let Some(hex) = body.strip_prefix('#') {
            let (hex, tint) = match hex.strip_suffix('*') {
                Some(h) => (h, Tint::Other),
                None => (hex, Tint::None),
            };
            if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(err());
            }
            if body.contains(['+', '-']) {
                return Err(format!("{v:?}: lighter and darker (+NN%, -NN%) apply to theme colours, not to #RRGGBB"));
            }
            let b = |k: usize| u8::from_str_radix(&hex[k..k + 2], 16).unwrap();
            return Ok(Color { base: ColorBase::Rgb([b(0), b(2), b(4)]), tint, alpha });
        }
        let name_end = body.find(['+', '-', '*']).unwrap_or(body.len());
        let (name, rest) = body.split_at(name_end);
        if !THEME_COLORS.contains(&name) {
            return Err(err());
        }
        let tint = match rest {
            "" => Tint::None,
            "*" => Tint::Other,
            r => {
                let (sign, n) = r.split_at(1);
                let n: u8 = n
                    .strip_suffix('%')
                    .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                    .and_then(|n| n.parse().ok())
                    .ok_or_else(err)?;
                if !(1..=99).contains(&n) {
                    return Err(format!("{v:?}: a theme colour is lighter or darker by 1% to 99%"));
                }
                if sign == "+" {
                    Tint::Lighter(n)
                } else {
                    Tint::Darker(n)
                }
            }
        };
        Ok(Color { base: ColorBase::Theme(name.to_string()), tint, alpha })
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.base {
            ColorBase::Rgb([r, g, b]) => write!(f, "#{r:02X}{g:02X}{b:02X}")?,
            ColorBase::Theme(n) => write!(f, "{n}")?,
        }
        match self.tint {
            Tint::None => {}
            Tint::Lighter(n) => write!(f, "+{n}%")?,
            Tint::Darker(n) => write!(f, "-{n}%")?,
            Tint::Other => write!(f, "*")?,
        }
        if let Some(a) = self.alpha {
            write!(f, "/{a}%")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------- fills

/// A fill: none, a colour, or a gradient, pattern or picture (shown, kept
/// while left as written, never written anew).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Fill {
    None,
    Color(Color),
    Gradient,
    Pattern,
    Picture,
}

impl Fill {
    pub fn parse(v: &str) -> Result<Fill, String> {
        Ok(match v.trim() {
            "none" => Fill::None,
            "gradient" => Fill::Gradient,
            "pattern" => Fill::Pattern,
            "picture" => Fill::Picture,
            c => Fill::Color(
                Color::parse(c)
                    .map_err(|_| format!("{v:?} is not a fill; a fill is none or a colour: {COLOR_FORM}"))?,
            ),
        })
    }

    /// Whether a text may write this fill anew.
    pub fn is_writable(&self) -> bool {
        match self {
            Fill::None => true,
            Fill::Color(c) => c.is_writable(),
            _ => false,
        }
    }
}

impl fmt::Display for Fill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fill::None => write!(f, "none"),
            Fill::Color(c) => write!(f, "{c}"),
            Fill::Gradient => write!(f, "gradient"),
            Fill::Pattern => write!(f, "pattern"),
            Fill::Picture => write!(f, "picture"),
        }
    }
}

// ---------------------------------------------------------------- borders

/// Border styles: the writable ones first, then those only shown.
pub const BORDER_STYLES: &[&str] = &["solid", "dashed", "dotted", "double", "dash-dot", "dash-dot-dot"];
pub const SHOWN_BORDER_STYLES: &[&str] = &["triple", "thin-thick", "thick-thin", "wave", "3d"];

/// A border (an outline, a cell side): none, or a line.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Border {
    None,
    Line {
        /// Hundredths of a point, above 0.
        width: i64,
        /// One of [`BORDER_STYLES`] or [`SHOWN_BORDER_STYLES`].
        style: String,
        color: Color,
    },
}

impl Border {
    pub fn parse(v: &str) -> Result<Border, String> {
        let s = v.trim();
        if s == "none" {
            return Ok(Border::None);
        }
        let form = "a border is none, or \"<width>pt <style> <colour>\" such as \"0.5pt solid #000000\", its style solid, dashed, dotted, double, dash-dot or dash-dot-dot";
        let parts: Vec<&str> = s.split_whitespace().collect();
        let [w, st, c] = parts.as_slice() else { return Err(format!("{v:?} is not a border; {form}")) };
        let width = parse_length(w, false).map_err(|m| format!("{m}; {form}"))?;
        if width == 0 {
            return Err(format!("{v:?}: a border's width is above 0pt; write none for no border"));
        }
        if !BORDER_STYLES.contains(st) && !SHOWN_BORDER_STYLES.contains(st) {
            return Err(format!("{v:?}: {st:?} is not a border style; {form}"));
        }
        Ok(Border::Line { width, style: st.to_string(), color: Color::parse(c)? })
    }

    /// Whether a text may write this border anew.
    pub fn is_writable(&self) -> bool {
        match self {
            Border::None => true,
            Border::Line { style, color, .. } => BORDER_STYLES.contains(&style.as_str()) && color.is_writable(),
        }
    }
}

impl fmt::Display for Border {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Border::None => write!(f, "none"),
            Border::Line { width, style, color } => write!(f, "{} {style} {color}", length_text(*width)),
        }
    }
}

// ---------------------------------------------------------------- paragraph values

pub const ALIGN: &[&str] = &["left", "center", "right", "justify", "distribute"];
pub const VALIGN: &[&str] = &["top", "middle", "bottom"];

/// A value that must be one of `allowed`.
pub fn parse_choice(key: &str, v: &str, allowed: &[&str]) -> Result<String, String> {
    if allowed.contains(&v) {
        Ok(v.to_string())
    } else {
        Err(format!("{key}={v} is not one of {}", allowed.join(", ")))
    }
}

/// Line spacing: proportional, exact, at least, or a gap between lines
/// (hwpx's 여백만 지정).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LineSpacing {
    /// Hundredths of a percent (16000 is 160%).
    Percent(i64),
    /// Hundredths of a point.
    Exact(i64),
    AtLeast(i64),
    Gap(i64),
}

impl LineSpacing {
    pub fn parse(v: &str) -> Result<LineSpacing, String> {
        let s = v.trim();
        let form =
            "line spacing is a percent (160%), a length for exact spacing (14pt), \"at-least 14pt\" or \"gap 4pt\"";
        if let Some(p) = s.strip_suffix('%') {
            let n = parse_length(p, false).map_err(|_| format!("{v:?} is not a line spacing; {form}"))?;
            return Ok(LineSpacing::Percent(n));
        }
        if let Some(l) = s.strip_prefix("at-least") {
            return Ok(LineSpacing::AtLeast(parse_length(l, false).map_err(|m| format!("{m}; {form}"))?));
        }
        if let Some(l) = s.strip_prefix("gap") {
            return Ok(LineSpacing::Gap(parse_length(l, false).map_err(|m| format!("{m}; {form}"))?));
        }
        Ok(LineSpacing::Exact(parse_length(s, false).map_err(|m| format!("{m}; {form}"))?))
    }
}

impl fmt::Display for LineSpacing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineSpacing::Percent(p) => write!(f, "{}%", length_text(*p).trim_end_matches("pt")),
            LineSpacing::Exact(l) => write!(f, "{}", length_text(*l)),
            LineSpacing::AtLeast(l) => write!(f, "at-least {}", length_text(*l)),
            LineSpacing::Gap(l) => write!(f, "gap {}", length_text(*l)),
        }
    }
}

/// A flag's value: bare (`bold`) is on, `bold=no` is off.
pub fn parse_flag(key: &str, v: Option<&str>) -> Result<bool, String> {
    match v {
        None | Some("yes") => Ok(true),
        Some("no") => Ok(false),
        Some(other) => Err(format!("{key}={other}: write {key} alone to turn it on, or {key}=no to turn it off")),
    }
}

// ---------------------------------------------------------------- attribute lists

/// A value as an attribute writes it: bare, or quoted when it is empty or
/// holds a space, a brace, a quote, `<`, `>` or `:`.
pub fn quote(v: &str) -> String {
    let bare =
        !v.is_empty() && !v.chars().any(|c| c.is_whitespace() || matches!(c, '{' | '}' | '"' | '\'' | '>' | ':' | '<'));
    if bare {
        v.to_string()
    } else {
        format!("\"{}\"", v.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;"))
    }
}

/// One `key=value` of a list: the key, its value (none for a bare flag),
/// and the key's column (from 1).
pub type Attr = (String, Option<String>, usize);

/// `key=value key="v w" flag` → its [`Attr`]s. The error is at a column.
pub fn attrs(src: &str) -> Result<Vec<Attr>, (usize, String)> {
    let cs: Vec<char> = src.chars().collect();
    let mut out = vec![];
    let mut k = 0;
    while k < cs.len() {
        if cs[k].is_whitespace() {
            k += 1;
            continue;
        }
        let start = k;
        while k < cs.len() && (cs[k].is_ascii_alphanumeric() || cs[k] == '-' || cs[k] == '_') {
            k += 1;
        }
        let key: String = cs[start..k].iter().collect();
        if key.is_empty() {
            return Err((start + 1, format!("{:?} is not a key=value pair; write key=value pairs separated by spaces, with quotes around a value that holds spaces", cs[start..].iter().take(20).collect::<String>())));
        }
        if cs.get(k) != Some(&'=') {
            if k < cs.len() && !cs[k].is_whitespace() {
                return Err((k + 1, format!("after {key}, write = and its value, such as {key}=value")));
            }
            out.push((key, None, start + 1));
            continue;
        }
        k += 1;
        let value = match cs.get(k) {
            Some(q @ ('"' | '\'')) => {
                let q = *q;
                let vs = k + 1;
                let Some(e) = (vs..cs.len()).find(|&j| cs[j] == q) else {
                    return Err((k + 1, format!("the value of {key} is not closed by {q}")));
                };
                k = e + 1;
                cs[vs..e].iter().collect::<String>()
            }
            _ => {
                let vs = k;
                while k < cs.len() && !cs[k].is_whitespace() && !matches!(cs[k], '"' | '\'') {
                    k += 1;
                }
                if k == vs {
                    return Err((k + 1, format!("{key}= has no value")));
                }
                cs[vs..k].iter().collect::<String>()
            }
        };
        let value = value.replace("&quot;", "\"").replace("&lt;", "<").replace("&amp;", "&");
        out.push((key, Some(value), start + 1));
    }
    Ok(out)
}

/// The refusal for a CSS-like value (`style="color:red"`): the vocabulary is
/// `key=value` pairs, and these are its keys.
pub fn css_refusal() -> String {
    format!(
        "formatting is key=value pairs, not CSS: write color=#FF0000, size=12pt. The keys are {}",
        order().collect::<Vec<_>>().join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(v: &str) -> String {
        Color::parse(v).unwrap_or_else(|e| panic!("{e}")).to_string()
    }

    #[test]
    fn colours_read_and_write_in_one_spelling() {
        assert_eq!(color("#fff0c3"), "#FFF0C3");
        assert_eq!(color("accent1"), "accent1");
        assert_eq!(color("bg1-90%"), "bg1-90%");
        assert_eq!(color("tx2+90%"), "tx2+90%");
        assert_eq!(color("accent1*"), "accent1*");
        assert_eq!(color("#101B3A/55%"), "#101B3A/55%");
        assert_eq!(color("accent2/100%"), "accent2");
        assert_eq!(color("accent1+40%/50%"), "accent1+40%/50%");
        assert_eq!(color("folHlink"), "folHlink");
        assert!(!Color::parse("accent1*").unwrap().is_writable());
        assert!(Color::parse("accent1+40%").unwrap().is_writable());
        for bad in ["red", "#FFF", "accent7", "text1", "accent1+0%", "accent1+100%", "#FF0000+20%", "accent1/120%", ""]
        {
            assert!(Color::parse(bad).is_err(), "{bad}");
        }
        assert_eq!(Color::parse("#1F4E79").unwrap(), Color::rgb(0x1F, 0x4E, 0x79));
        assert_eq!(Color::parse("accent2").unwrap(), Color::theme("accent2"));
    }

    #[test]
    fn lengths_borders_fills_and_spacing() {
        assert_eq!(parse_length("0.34pt", false), Ok(34));
        assert_eq!(parse_length("12", false), Ok(1200));
        assert_eq!(parse_length("-58.96pt", true), Ok(-5896));
        assert!(parse_length("-1pt", false).is_err() && parse_length("12px", false).is_err());
        assert_eq!(length_text(34), "0.34pt");
        assert_eq!(length_text(283), "2.83pt");
        assert_eq!(length_text(-5896), "-58.96pt");
        assert_eq!(length_text(2000), "20pt");
        assert_eq!(length_text(150), "1.5pt");
        let b = Border::parse("2.83pt solid #7f7f7f").unwrap();
        assert_eq!(b.to_string(), "2.83pt solid #7F7F7F");
        assert_eq!(Border::parse("2pt solid accent1*").unwrap().to_string(), "2pt solid accent1*");
        assert!(!Border::parse("2pt solid accent1*").unwrap().is_writable());
        assert!(!Border::parse("1pt triple #000000").unwrap().is_writable());
        assert_eq!(Border::parse("none"), Ok(Border::None));
        assert!(Border::parse("0pt solid #000000").is_err() && Border::parse("1pt groove #000000").is_err());
        assert_eq!(Fill::parse("gradient").unwrap().to_string(), "gradient");
        assert!(!Fill::parse("pattern").unwrap().is_writable());
        assert_eq!(Fill::parse("accent1+80%").unwrap().to_string(), "accent1+80%");
        for (v, c) in [
            ("160%", "160%"),
            ("90%", "90%"),
            ("14pt", "14pt"),
            ("at-least 14pt", "at-least 14pt"),
            ("gap 4pt", "gap 4pt"),
        ] {
            assert_eq!(LineSpacing::parse(v).unwrap().to_string(), c);
        }
        assert_eq!(parse_flag("bold", None), Ok(true));
        assert_eq!(parse_flag("bold", Some("no")), Ok(false));
        assert!(parse_flag("bold", Some("maybe")).is_err());
    }

    #[test]
    fn attribute_lists_read_bare_and_quoted_values() {
        let a = attrs(r#"fill=#FFF0C3 border-bottom="2.83pt solid #7F7F7F" bold font="맑은 고딕""#).unwrap();
        let v: Vec<(&str, Option<&str>)> = a.iter().map(|x| (x.0.as_str(), x.1.as_deref())).collect();
        assert_eq!(
            v,
            [
                ("fill", Some("#FFF0C3")),
                ("border-bottom", Some("2.83pt solid #7F7F7F")),
                ("bold", None),
                ("font", Some("맑은 고딕"))
            ]
        );
        assert_eq!(a[1].2, 14);
        assert!(attrs("size=").is_err() && attrs("font=\"x").is_err() && attrs("=3").is_err());
        assert_eq!(quote("맑은 고딕"), "\"맑은 고딕\"");
        assert_eq!(quote("#101B3A/55%"), "#101B3A/55%");
        assert_eq!(rank("fill"), Some(0));
        assert!(rank("align") < rank("font") && rank("font") < rank("size") && rank("size") < rank("color"));
        assert!(css_refusal().contains("fill, border"));
    }
}
