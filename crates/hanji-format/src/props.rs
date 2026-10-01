//! Property sets of the direct-formatting vocabulary (DESIGN.md §5.1): the
//! values of [`crate::vocab`] keyed by their property, parsed from a
//! `key=value` list and written back in canonical order (`border` folded
//! when the four sides are equal). Where a set attaches is each kind's
//! grammar (§5.2: a style line, `{…}`, `[text]{…}`, a cell, a row, a table
//! line).

use std::collections::BTreeMap;
use std::fmt;

use crate::vocab::{self, Border, Color, Fill, LineSpacing};

/// A property of the vocabulary, in the order canonical form writes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    Fill,
    BorderTop,
    BorderRight,
    BorderBottom,
    BorderLeft,
    Valign,
    Align,
    IndentLeft,
    IndentRight,
    FirstLine,
    SpaceBefore,
    SpaceAfter,
    LineSpacing,
    Font,
    Size,
    Color,
    Bold,
    Italic,
    Underline,
    Strike,
    Indent,
}

impl Key {
    pub const ALL: [Key; 21] = [
        Key::Fill,
        Key::BorderTop,
        Key::BorderRight,
        Key::BorderBottom,
        Key::BorderLeft,
        Key::Valign,
        Key::Align,
        Key::IndentLeft,
        Key::IndentRight,
        Key::FirstLine,
        Key::SpaceBefore,
        Key::SpaceAfter,
        Key::LineSpacing,
        Key::Font,
        Key::Size,
        Key::Color,
        Key::Bold,
        Key::Italic,
        Key::Underline,
        Key::Strike,
        Key::Indent,
    ];
    /// The four sides, as `border` sets them.
    pub const SIDES: [Key; 4] = [Key::BorderTop, Key::BorderRight, Key::BorderBottom, Key::BorderLeft];
    /// A paragraph's layout.
    pub const PARA: [Key; 7] = [
        Key::Align,
        Key::IndentLeft,
        Key::IndentRight,
        Key::FirstLine,
        Key::SpaceBefore,
        Key::SpaceAfter,
        Key::LineSpacing,
    ];
    /// Text properties that are not marks.
    pub const TEXT: [Key; 3] = [Key::Font, Key::Size, Key::Color];
    /// Flags: in running text they are the marks `**`, `*`, `<u>`, `~~`.
    pub const FLAGS: [Key; 4] = [Key::Bold, Key::Italic, Key::Underline, Key::Strike];
    /// A box: a cell, a shape.
    pub const BOX: [Key; 6] =
        [Key::Fill, Key::BorderTop, Key::BorderRight, Key::BorderBottom, Key::BorderLeft, Key::Valign];

    pub fn name(self) -> &'static str {
        match self {
            Key::Fill => "fill",
            Key::BorderTop => "border-top",
            Key::BorderRight => "border-right",
            Key::BorderBottom => "border-bottom",
            Key::BorderLeft => "border-left",
            Key::Valign => "valign",
            Key::Align => "align",
            Key::IndentLeft => "indent-left",
            Key::IndentRight => "indent-right",
            Key::FirstLine => "first-line",
            Key::SpaceBefore => "space-before",
            Key::SpaceAfter => "space-after",
            Key::LineSpacing => "line-spacing",
            Key::Font => "font",
            Key::Size => "size",
            Key::Color => "color",
            Key::Bold => "bold",
            Key::Italic => "italic",
            Key::Underline => "underline",
            Key::Strike => "strike",
            Key::Indent => "indent",
        }
    }

    pub fn from_name(n: &str) -> Option<Key> {
        Key::ALL.into_iter().find(|k| k.name() == n)
    }

    pub fn is_flag(self) -> bool {
        Key::FLAGS.contains(&self)
    }

    pub fn is_side(self) -> bool {
        Key::SIDES.contains(&self)
    }

    /// The value of this key as a text writes it.
    pub fn parse(self, v: Option<&str>) -> Result<Value, String> {
        let name = self.name();
        if self.is_flag() {
            return vocab::parse_flag(name, v).map(Value::Flag);
        }
        let Some(v) = v else { return Err(format!("{name} needs a value: {name}=…")) };
        let at = |m: String| format!("{name}: {m}");
        Ok(match self {
            Key::Fill => Value::Fill(Fill::parse(v).map_err(at)?),
            Key::BorderTop | Key::BorderRight | Key::BorderBottom | Key::BorderLeft => {
                Value::Border(Border::parse(v).map_err(at)?)
            }
            Key::Valign => Value::Choice(vocab::parse_choice(name, v, vocab::VALIGN)?),
            Key::Align => Value::Choice(vocab::parse_choice(name, v, vocab::ALIGN)?),
            Key::IndentLeft | Key::IndentRight | Key::FirstLine | Key::SpaceBefore | Key::SpaceAfter => {
                let neg = matches!(self, Key::IndentLeft | Key::IndentRight | Key::FirstLine);
                Value::Len(vocab::parse_length(v, neg).map_err(at)?)
            }
            Key::Size => {
                let n = vocab::parse_length(v, false).map_err(at)?;
                if n == 0 {
                    return Err(format!("size={v}: a font size is above 0pt"));
                }
                Value::Len(n)
            }
            Key::LineSpacing => Value::Spacing(LineSpacing::parse(v).map_err(at)?),
            Key::Font => {
                if v.trim().is_empty() {
                    return Err("font needs a font name: font=\"맑은 고딕\"".into());
                }
                Value::Text(v.trim().to_string())
            }
            Key::Color => Value::Color(Color::parse(v).map_err(at)?),
            Key::Indent => {
                let n: u32 = v
                    .parse()
                    .ok()
                    .filter(|n| *n <= 250)
                    .ok_or_else(|| format!("indent={v}: a whole number of indent levels, 0 to 250"))?;
                Value::Levels(n)
            }
            Key::Bold | Key::Italic | Key::Underline | Key::Strike => unreachable!(),
        })
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A property's value.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Value {
    /// Hundredths of a point.
    Len(i64),
    Color(Color),
    Fill(Fill),
    Border(Border),
    Spacing(LineSpacing),
    /// One of a key's fixed words (`align`, `valign`).
    Choice(String),
    Flag(bool),
    /// A font name.
    Text(String),
    /// Spreadsheet indent levels.
    Levels(u32),
}

impl Value {
    /// Whether a text may write this value anew: not a shown-only fill,
    /// border style or `*` colour.
    pub fn is_writable(&self) -> bool {
        match self {
            Value::Color(c) => c.is_writable(),
            Value::Fill(f) => f.is_writable(),
            Value::Border(b) => b.is_writable(),
            _ => true,
        }
    }

    pub fn length(&self) -> Option<i64> {
        match self {
            Value::Len(n) => Some(*n),
            _ => None,
        }
    }

    pub fn flag(&self) -> Option<bool> {
        match self {
            Value::Flag(b) => Some(*b),
            _ => None,
        }
    }

    pub fn choice(&self) -> Option<&str> {
        match self {
            Value::Choice(s) | Value::Text(s) => Some(s),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Len(n) => f.write_str(&vocab::length_text(*n)),
            Value::Color(c) => write!(f, "{c}"),
            Value::Fill(x) => write!(f, "{x}"),
            Value::Border(b) => write!(f, "{b}"),
            Value::Spacing(s) => write!(f, "{s}"),
            Value::Choice(s) | Value::Text(s) => f.write_str(s),
            Value::Flag(b) => f.write_str(if *b { "yes" } else { "no" }),
            Value::Levels(n) => write!(f, "{n}"),
        }
    }
}

/// A set of properties: each key at most once.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Props(pub BTreeMap<Key, Value>);

impl Props {
    pub fn new() -> Props {
        Props::default()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, k: Key) -> Option<&Value> {
        self.0.get(&k)
    }

    pub fn has(&self, k: Key) -> bool {
        self.0.contains_key(&k)
    }

    pub fn set(&mut self, k: Key, v: Value) {
        self.0.insert(k, v);
    }

    pub fn remove(&mut self, k: Key) -> Option<Value> {
        self.0.remove(&k)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Key, &Value)> {
        self.0.iter().map(|(k, v)| (*k, v))
    }

    pub fn keys(&self) -> impl Iterator<Item = Key> + '_ {
        self.0.keys().copied()
    }

    /// The properties of `keys` only.
    pub fn only(&self, keys: &[Key]) -> Props {
        Props(self.0.iter().filter(|(k, _)| keys.contains(k)).map(|(k, v)| (*k, v.clone())).collect())
    }

    /// Without the properties of `keys`.
    pub fn without(&self, keys: &[Key]) -> Props {
        Props(self.0.iter().filter(|(k, _)| !keys.contains(k)).map(|(k, v)| (*k, v.clone())).collect())
    }

    /// `self` with `over`'s properties on top.
    pub fn overlay(&self, over: &Props) -> Props {
        let mut out = self.clone();
        for (k, v) in &over.0 {
            out.0.insert(*k, v.clone());
        }
        out
    }

    /// The properties of `self` that `base` does not have with the same value.
    pub fn diff(&self, base: &Props) -> Props {
        Props(self.0.iter().filter(|(k, v)| base.0.get(k) != Some(v)).map(|(k, v)| (*k, v.clone())).collect())
    }

    /// Canonical text, no braces: canonical key order, `border` for four
    /// equal sides, a value quoted when it holds a space.
    pub fn write(&self) -> String {
        let sides: Vec<Option<&Value>> = Key::SIDES.iter().map(|k| self.get(*k)).collect();
        let fold = sides.iter().all(Option::is_some) && sides.windows(2).all(|w| w[0] == w[1]);
        let mut out: Vec<String> = vec![];
        for (k, v) in self.iter() {
            if fold && k.is_side() {
                if k == Key::BorderTop {
                    out.push(format!("border={}", vocab::quote(&v.to_string())));
                }
                continue;
            }
            out.push(match v {
                Value::Flag(true) => k.name().to_string(),
                Value::Flag(false) => format!("{}=no", k.name()),
                v => format!("{}={}", k.name(), vocab::quote(&v.to_string())),
            });
        }
        out.join(" ")
    }
}

impl fmt::Display for Props {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.write())
    }
}

impl serde::Serialize for Props {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.write())
    }
}

impl<'de> serde::Deserialize<'de> for Props {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Props, D::Error> {
        let s = String::deserialize(d)?;
        let (props, extra) = parse_list(&s, &[]).map_err(|(_, m)| serde::de::Error::custom(m))?;
        if let Some((k, ..)) = extra.first() {
            return Err(serde::de::Error::custom(format!("{k} is not a property")));
        }
        Ok(props)
    }
}

/// One entry of a written list: a property (with its value parsed), or one
/// of the names the caller takes itself (`style`, `name`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Prop(Key, Value),
    Other(String, Option<String>),
}

/// A written list as its entries, each with its column (from 1); `border`
/// expands to the four sides. Names in `extra` come back as
/// [`Item::Other`]; any other name that is not a key is an error that says
/// what to write instead.
pub fn items(src: &str, extra: &[&str]) -> Result<Vec<(Item, usize)>, (usize, String)> {
    let attrs = vocab::attrs(src)?;
    let mut out = vec![];
    for (name, value, col) in attrs {
        if extra.contains(&name.as_str()) {
            out.push((Item::Other(name, value), col));
            continue;
        }
        if name == "border" {
            let b = Key::BorderTop.parse(value.as_deref()).map_err(|m| (col, m.replacen("border-top", "border", 1)))?;
            for k in Key::SIDES {
                out.push((Item::Prop(k, b.clone()), col));
            }
            continue;
        }
        let Some(key) = Key::from_name(&name) else { return Err((col, unknown(&name))) };
        let v = key.parse(value.as_deref()).map_err(|m| (col, m))?;
        out.push((Item::Prop(key, v), col));
    }
    Ok(out)
}

/// [`items`] as a set: a property written twice is an error (`border` and a
/// side of it too).
#[allow(clippy::type_complexity)]
pub fn parse_list(src: &str, extra: &[&str]) -> Result<(Props, Vec<(String, Option<String>, usize)>), (usize, String)> {
    let mut props = Props::new();
    let mut others = vec![];
    for (it, col) in items(src, extra)? {
        match it {
            Item::Prop(k, v) => {
                if props.has(k) {
                    let what = if k.is_side() { format!("{k} (or border)") } else { k.to_string() };
                    return Err((col, format!("{what} is written twice; write each property once")));
                }
                props.set(k, v);
            }
            Item::Other(n, v) => {
                if others.iter().any(|(x, _, _): &(String, Option<String>, usize)| *x == n) {
                    return Err((col, format!("{n} is written twice")));
                }
                others.push((n, v, col));
            }
        }
    }
    Ok((props, others))
}

/// Why `name` is not a property: what it is called here, or why it cannot
/// be written (DESIGN.md §5.1: refused, not expressible).
pub fn unknown(name: &str) -> String {
    let keys = "fill, border, border-top, border-right, border-bottom, border-left, valign, align, indent-left, indent-right, first-line, space-before, space-after, line-spacing, font, size, color, bold, italic, underline, strike";
    let instead = match name {
        "background" | "background-color" | "bg" | "shading" | "shade" | "fill-color" => Some("fill"),
        "colour" | "font-color" | "text-color" | "fg" | "foreground" => Some("color"),
        "font-size" | "fontsize" | "sz" | "pt" => Some("size"),
        "font-family" | "face" | "typeface" | "font-face" => Some("font"),
        "text-align" | "alignment" | "justify" | "halign" => Some("align"),
        "vertical-align" | "vertical-alignment" | "v-align" => Some("valign"),
        "margin-left" | "left-indent" | "indent" | "padding-left" => Some("indent-left"),
        "margin-right" | "right-indent" | "padding-right" => Some("indent-right"),
        "text-indent" | "first-line-indent" | "hanging" | "hanging-indent" => {
            Some("first-line (negative for a hanging indent)")
        }
        "margin-top" | "spacing-before" | "space-above" => Some("space-before"),
        "margin-bottom" | "spacing-after" | "space-below" => Some("space-after"),
        "line-height" | "leading" | "linespacing" | "line" => Some("line-spacing"),
        "font-weight" | "strong" | "b" => Some("bold (or **text**)"),
        "font-style" | "em" | "i" => Some("italic (or *text*)"),
        "text-decoration" | "u" => Some("underline (or <u>text</u>)"),
        "strikethrough" | "line-through" | "s" => Some("strike (or ~~text~~)"),
        "outline" | "stroke" | "line-color" => Some("border"),
        _ => None,
    };
    if let Some(k) = instead {
        return format!("{name} is not a property; write {k}. The properties are {keys}");
    }
    if matches!(name, "table-align" | "table-indent") {
        return format!("{name} is a table's own position: it goes on the table line {{…}} directly before the table's header row, not here");
    }
    let refused = match name {
        "highlight" | "mark" => Some("text highlight"),
        "shadow" | "glow" | "effect" | "effects" | "reflection" | "emboss" | "engrave" | "3d" => {
            Some("shadows, glow and other effects")
        }
        "spacing" | "letter-spacing" | "char-spacing" | "character-spacing" | "kerning" | "tracking" | "width"
        | "scale" | "ratio" => Some("character spacing and width"),
        "gradient" | "pattern" | "hatch" | "texture" => Some("gradients and patterns"),
        "diagonal" | "border-diagonal" | "slash" | "backslash" | "diagonal-up" | "diagonal-down" => {
            Some("diagonal cell borders")
        }
        "rotate" | "rotation" | "direction" | "text-direction" | "writing-mode" => Some("text direction"),
        "caps" | "small-caps" | "superscript" | "subscript" | "vertical" | "baseline" => {
            Some("capitals, superscript and subscript")
        }
        _ => None,
    };
    if let Some(what) = refused {
        return format!("{name}: {what} cannot be written (not in the vocabulary); the file keeps what it has. The properties are {keys}");
    }
    format!("{name} is not a property. The properties are {keys}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(s: &str) -> Props {
        parse_list(s, &[]).unwrap_or_else(|e| panic!("{s}: {e:?}")).0
    }

    #[test]
    fn a_list_reads_and_writes_in_canonical_order() {
        let p = props(r#"size=20pt align=center fill=#fff0c3 border-bottom="2.83pt solid #7F7F7F" font=나눔고딕 bold"#);
        assert_eq!(
            p.write(),
            r#"fill=#FFF0C3 border-bottom="2.83pt solid #7F7F7F" align=center font=나눔고딕 size=20pt bold"#
        );
        let b = props(r#"border="0.5pt solid bg1-90%""#);
        assert_eq!(b.write(), r#"border="0.5pt solid bg1-90%""#);
        assert_eq!(b.len_of_sides(), 4);
        let mut c = b.clone();
        c.set(Key::BorderLeft, Value::Border(Border::None));
        assert_eq!(
            c.write(),
            r#"border-top="0.5pt solid bg1-90%" border-right="0.5pt solid bg1-90%" border-bottom="0.5pt solid bg1-90%" border-left=none"#
        );
        assert_eq!(
            props("italic=no line-spacing=\"at-least 14pt\"").write(),
            "line-spacing=\"at-least 14pt\" italic=no"
        );
        assert_eq!(props("first-line=-10pt").get(Key::FirstLine), Some(&Value::Len(-1000)));
    }

    #[test]
    fn a_list_names_what_is_wrong() {
        let err = |s: &str| parse_list(s, &[]).unwrap_err();
        assert!(err("size=12pt size=13pt").1.contains("twice"));
        assert!(err("border=none border-top=none").1.contains("twice"));
        assert!(err("background=#FF0000").1.contains("write fill"));
        assert!(err("highlight=yellow").1.contains("cannot be written"));
        assert!(err("align=middle").1.contains("left, center"));
        assert!(err("size=0pt").1.contains("above 0pt"));
        assert!(err("space-before=-2pt").1.contains("0pt or more"));
        assert_eq!(err("fill=#FFF0C3 color=red").0, 14);
        let (p, other) = parse_list(r#"style="개요 3" first-line=10pt"#, &["style"]).unwrap();
        assert_eq!(other, vec![("style".to_string(), Some("개요 3".to_string()), 1)]);
        assert_eq!(p.write(), "first-line=10pt");
    }

    #[test]
    fn sets_compare_and_combine() {
        let base = props("align=justify size=10pt font=바탕");
        let own = props("size=15pt bold");
        let eff = base.overlay(&own);
        assert_eq!(eff.write(), "align=justify font=바탕 size=15pt bold");
        assert_eq!(eff.diff(&base).write(), "size=15pt bold");
        assert_eq!(eff.only(&Key::PARA).write(), "align=justify");
        assert_eq!(eff.without(&Key::FLAGS).write(), "align=justify font=바탕 size=15pt");
        let json = serde_json::to_string(&eff).unwrap();
        assert_eq!(json, "\"align=justify font=바탕 size=15pt bold\"");
        assert_eq!(serde_json::from_str::<Props>(&json).unwrap(), eff);
        assert!(!props("fill=gradient").get(Key::Fill).unwrap().is_writable());
    }

    impl Props {
        fn len_of_sides(&self) -> usize {
            Key::SIDES.iter().filter(|k| self.has(**k)).count()
        }
    }
}
