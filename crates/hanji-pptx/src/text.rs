//! Text formatting (DESIGN.md §5.3): the font, size and colour a run shows,
//! read through what it inherits (the presentation's default text style,
//! the master's text styles, the master's and layout's placeholder, the
//! shape's style and list style, the run), and written back into the run's
//! `a:rPr` only where the text changed it.

use serde::{Deserialize, Serialize};

use hanji_format::inline_style::{self as style, TextStyle};
use hanji_package::xml::{fragment, insert_ordered, Element, Node};

/// What a run's `a:rPr` (or a list level's `a:defRPr`) sets of the text
/// formatting: its Latin and East Asian fonts (theme fonts resolved), its
/// size in hundredths of a point, its colour (canonical, see
/// [`style::parse_color`]).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ea: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// For a colour the text cannot write ([`is_kept`]): the fill as the
    /// file stores it, so a run can keep it where it no longer inherits it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
}

/// A theme's fonts (`a:fontScheme`): major (headings) and minor (body),
/// Latin and East Asian; an empty typeface is none.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeFonts {
    pub major_latin: Option<String>,
    pub major_ea: Option<String>,
    pub minor_latin: Option<String>,
    pub minor_ea: Option<String>,
}

impl ThemeFonts {
    /// A theme's font scheme.
    pub fn of(theme: &Element) -> ThemeFonts {
        let mut f = ThemeFonts::default();
        theme.walk(&mut |e| {
            if e.is("a:fontScheme") {
                let face = |g: &str, k: &str| {
                    e.child(g).and_then(|x| x.child(k)).and_then(|x| x.get("typeface")).filter(|t| !t.is_empty())
                };
                f = ThemeFonts {
                    major_latin: face("a:majorFont", "a:latin"),
                    major_ea: face("a:majorFont", "a:ea"),
                    minor_latin: face("a:minorFont", "a:latin"),
                    minor_ea: face("a:minorFont", "a:ea"),
                };
            }
        });
        f
    }

    /// A typeface as a run names it: `+mn-lt` and the like are the theme's.
    fn resolve(&self, t: &str) -> Option<String> {
        let v = match t {
            "+mj-lt" => self.major_latin.clone(),
            "+mj-ea" => self.major_ea.clone(),
            "+mn-lt" => self.minor_latin.clone(),
            "+mn-ea" => self.minor_ea.clone(),
            "+mj-cs" | "+mn-cs" => None,
            t => Some(t.to_string()),
        };
        v.filter(|x| !x.is_empty())
    }
}

impl RunStyle {
    /// `self`'s values, and `under`'s where `self` has none.
    pub fn over(&self, under: &RunStyle) -> RunStyle {
        RunStyle {
            latin: self.latin.clone().or_else(|| under.latin.clone()),
            ea: self.ea.clone().or_else(|| under.ea.clone()),
            size: self.size.or(under.size),
            color: self.color.clone().or_else(|| under.color.clone()),
            fill: if self.color.is_some() { self.fill.clone() } else { under.fill.clone() },
        }
    }

    /// As the text shows it: the East Asian font when there is one, else
    /// the Latin one (round 6: `font` is the Korean text's).
    pub fn shown(&self) -> TextStyle {
        TextStyle { font: self.ea.clone().or_else(|| self.latin.clone()), size: self.size, color: self.color.clone() }
    }

    /// What an `a:rPr` or `a:defRPr` sets.
    pub fn of(rpr: &Element, fonts: &ThemeFonts) -> RunStyle {
        let face = |k: &str| rpr.child(k).and_then(|x| x.get("typeface")).and_then(|t| fonts.resolve(&t));
        RunStyle {
            latin: face("a:latin"),
            ea: face("a:ea"),
            size: rpr.get("sz").and_then(|v| v.trim().parse().ok()),
            color: fill_color(rpr),
            fill: fill_color(rpr)
                .filter(|c| is_kept(c))
                .and_then(|_| rpr.elements().find(|e| FILLS.contains(&e.name.as_str())))
                .map(|f| f.to_xml()),
        }
    }
}

/// What a run shows when nothing sets it: the theme's minor fonts, 18 pt, `tx1`.
pub fn defaults(fonts: &ThemeFonts) -> RunStyle {
    RunStyle {
        latin: fonts.minor_latin.clone(),
        ea: fonts.minor_ea.clone(),
        size: Some(1800),
        color: Some("tx1".into()),
        fill: None,
    }
}

/// Per level (0–8), what a list style (`a:lstStyle`, `p:titleStyle`, …)
/// sets: its `a:defPPr` beneath each `a:lvlNpPr`'s `a:defRPr`.
pub fn levels(lst: Option<&Element>, fonts: &ThemeFonts) -> [RunStyle; 9] {
    let mut out: [RunStyle; 9] = Default::default();
    let Some(l) = lst else { return out };
    let of =
        |p: Option<&Element>| p.and_then(|p| p.child("a:defRPr")).map(|r| RunStyle::of(r, fonts)).unwrap_or_default();
    let def = of(l.child("a:defPPr"));
    for (k, o) in out.iter_mut().enumerate() {
        *o = of(l.child(&format!("a:lvl{}pPr", k + 1))).over(&def);
    }
    out
}

/// Per level, `top` over `under`.
pub fn over(top: &[RunStyle; 9], under: &[RunStyle; 9]) -> [RunStyle; 9] {
    let mut out = under.clone();
    for k in 0..9 {
        out[k] = top[k].over(&under[k]);
    }
    out
}

/// What a shape's text inherits at each level: `inherited` (its layout
/// slot's, or the deck's for a shape that is not a placeholder) beneath its
/// style's font (`p:style/a:fontRef`) and its own list style.
pub fn base(el: &Element, inherited: &[RunStyle; 9], fonts: &ThemeFonts) -> [RunStyle; 9] {
    let mut under = inherited.clone();
    if let Some(fr) = el.child("p:style").and_then(|s| s.child("a:fontRef")) {
        let (latin, ea) = match fr.get("idx").as_deref() {
            Some("major") => (fonts.major_latin.clone(), fonts.major_ea.clone()),
            Some("minor") => (fonts.minor_latin.clone(), fonts.minor_ea.clone()),
            _ => (None, None),
        };
        let color = color_of(fr);
        let fill = color
            .as_ref()
            .filter(|c| is_kept(c))
            .and_then(|_| fr.elements().next())
            .map(|c| format!("<a:solidFill>{}</a:solidFill>", c.to_xml()));
        let r = RunStyle { latin, ea, size: None, color, fill };
        under =
            over(&[r.clone(), r.clone(), r.clone(), r.clone(), r.clone(), r.clone(), r.clone(), r.clone(), r], &under);
    }
    let own = levels(el.child("p:txBody").and_then(|t| t.child("a:lstStyle")), fonts);
    over(&own, &under)
}

// ---------------------------------------------------------------- colours

const FILLS: &[&str] = &["a:noFill", "a:solidFill", "a:gradFill", "a:blipFill", "a:pattFill", "a:grpFill"];

/// The colour an `a:rPr` fill gives its text: a colour, `none`, or what the
/// text shows but cannot write (`gradient`, `pattern`, `picture`).
fn fill_color(rpr: &Element) -> Option<String> {
    let f = rpr.elements().find(|e| FILLS.contains(&e.name.as_str()))?;
    Some(match f.name.as_str() {
        "a:noFill" => "none".into(),
        "a:solidFill" => color_of(f)?,
        "a:gradFill" => "gradient".into(),
        "a:pattFill" => "pattern".into(),
        "a:blipFill" => "picture".into(),
        _ => return None,
    })
}

/// The canonical colour of the first colour element in `holder`
/// (`a:solidFill`, `a:fontRef`, …).
pub fn color_of(holder: &Element) -> Option<String> {
    let c = holder.elements().find(|e| {
        matches!(e.name.as_str(), "a:srgbClr" | "a:schemeClr" | "a:sysClr" | "a:prstClr" | "a:scrgbClr" | "a:hslClr")
    })?;
    let v = c.get("val").unwrap_or_default();
    let base = match c.name.as_str() {
        "a:srgbClr" if v.len() == 6 && v.chars().all(|x| x.is_ascii_hexdigit()) => {
            format!("#{}", v.to_ascii_uppercase())
        }
        "a:schemeClr" if style::THEME_COLORS.contains(&v.as_str()) => v,
        "a:sysClr" => {
            let last = c.get("lastClr").filter(|l| l.len() == 6);
            let hex = last.unwrap_or_else(|| if v == "window" { "FFFFFF".into() } else { "000000".into() });
            format!("#{}", hex.to_ascii_uppercase())
        }
        "a:prstClr" => format!("#{}", preset_color(&v)?),
        "a:scrgbClr" => {
            let ch = |k: &str| c.get(k).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0) / 100_000.0;
            let s = |l: f64| {
                let l = l.clamp(0.0, 1.0);
                let v = if l <= 0.003_130_8 { 12.92 * l } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 };
                (v * 255.0).round() as u8
            };
            format!("#{:02X}{:02X}{:02X}", s(ch("r")), s(ch("g")), s(ch("b")))
        }
        "a:hslClr" => {
            let n = |k: &str| c.get(k).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0);
            let (h, s, l) = (n("hue") / 60_000.0 / 360.0, n("sat") / 100_000.0, n("lum") / 100_000.0);
            let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
            let p = 2.0 * l - q;
            let t = |mut t: f64| {
                t = t.rem_euclid(1.0);
                let v = if t < 1.0 / 6.0 {
                    p + (q - p) * 6.0 * t
                } else if t < 0.5 {
                    q
                } else if t < 2.0 / 3.0 {
                    p + (q - p) * (2.0 / 3.0 - t) * 6.0
                } else {
                    p
                };
                (v.clamp(0.0, 1.0) * 255.0).round() as u8
            };
            format!("#{:02X}{:02X}{:02X}", t(h + 1.0 / 3.0), t(h), t(h - 1.0 / 3.0))
        }
        _ => return None,
    };
    let mut out = base;
    let mut alpha = None;
    let (mut lum_mod, mut lum_off, mut other) = (None, None, false);
    for m in c.elements() {
        let v = m.get("val").and_then(|v| v.parse::<i64>().ok());
        match m.name.as_str() {
            "a:alpha" => alpha = v,
            "a:lumMod" if lum_mod.is_none() => lum_mod = v,
            "a:lumOff" if lum_off.is_none() => lum_off = v,
            _ => other = true,
        }
    }
    // Office's presets: lighter N% is lumMod 100−N, lumOff N; darker N% is lumMod 100−N.
    let pct = |v: i64| (v % 1000 == 0 && (1..=99).contains(&(v / 1000))).then_some(v / 1000);
    let tint = match (lum_mod, lum_off) {
        _ if other => Some("*".to_string()),
        (None, None) => None,
        (Some(m), Some(o)) if m + o == 100_000 && c.is("a:schemeClr") => {
            Some(pct(o).map_or("*".to_string(), |n| format!("+{n}%")))
        }
        (Some(m), None) if c.is("a:schemeClr") => Some(pct(100_000 - m).map_or("*".to_string(), |n| format!("-{n}%"))),
        _ => Some("*".to_string()),
    };
    if let Some(t) = tint {
        out.push_str(&t);
    }
    if let Some(a) = alpha {
        let pct = ((a as f64) / 1000.0).round() as i64;
        if pct < 100 {
            out.push_str(&format!("/{}%", pct.max(0)));
        }
    }
    Some(out)
}

/// `a:solidFill` (or `a:noFill`) for a canonical colour the text writes.
pub fn fill_xml(c: &str) -> String {
    if c == "none" {
        return "<a:noFill/>".into();
    }
    use hanji_format::vocab::{Color, ColorBase, Tint};
    let Ok(col) = Color::parse(c) else { return "<a:noFill/>".into() };
    let mut mods = match col.tint {
        Tint::Lighter(n) => {
            format!("<a:lumMod val=\"{}\"/><a:lumOff val=\"{}\"/>", (100 - n as i64) * 1000, n as i64 * 1000)
        }
        Tint::Darker(n) => format!("<a:lumMod val=\"{}\"/>", (100 - n as i64) * 1000),
        _ => String::new(),
    };
    if let Some(a) = col.alpha {
        mods.push_str(&format!("<a:alpha val=\"{}\"/>", a as i64 * 1000));
    }
    let (tag, val) = match &col.base {
        ColorBase::Rgb([r, g, b]) => ("a:srgbClr", format!("{r:02X}{g:02X}{b:02X}")),
        ColorBase::Theme(n) => ("a:schemeClr", n.clone()),
    };
    let el = if mods.is_empty() {
        format!("<{tag} val=\"{val}\"/>")
    } else {
        format!("<{tag} val=\"{val}\">{mods}</{tag}>")
    };
    format!("<a:solidFill>{el}</a:solidFill>")
}

/// `rpr` without what the text shows of it (size, fill, Latin and East
/// Asian fonts): what a run's fingerprint holds.
pub fn without_shown(rpr: &mut Element) {
    rpr.remove_attr("sz");
    rpr.children
        .retain(|n| !matches!(n, Node::El(e) if FILLS.contains(&e.name.as_str()) || e.is("a:latin") || e.is("a:ea")));
}

/// `rpr` takes `fill` (an `a:rPr` fill element's XML) in place of its own.
pub fn set_fill(rpr: &mut Element, fill: &str) {
    rpr.children.retain(|n| !matches!(n, Node::El(e) if FILLS.contains(&e.name.as_str())));
    insert_ordered(rpr, fragment(fill), RPR_ORDER);
}

/// A colour the text shows but cannot write: a stored adjustment (`*`), a
/// gradient, pattern or picture.
pub fn is_kept(c: &str) -> bool {
    style::is_kept_color(c) || matches!(c, "gradient" | "pattern" | "picture")
}

/// DrawingML's preset colours (`ST_PresetColorVal`) as hex.
fn preset_color(name: &str) -> Option<&'static str> {
    PRESET_COLORS.iter().find(|(n, _)| *n == name).map(|x| x.1)
}

#[rustfmt::skip]
const PRESET_COLORS: &[(&str, &str)] = &[
    ("aliceBlue", "F0F8FF"), ("antiqueWhite", "FAEBD7"), ("aqua", "00FFFF"), ("aquamarine", "7FFFD4"), ("azure", "F0FFFF"),
    ("beige", "F5F5DC"), ("bisque", "FFE4C4"), ("black", "000000"), ("blanchedAlmond", "FFEBCD"), ("blue", "0000FF"),
    ("blueViolet", "8A2BE2"), ("brown", "A52A2A"), ("burlyWood", "DEB887"), ("cadetBlue", "5F9EA0"), ("chartreuse", "7FFF00"),
    ("chocolate", "D2691E"), ("coral", "FF7F50"), ("cornflowerBlue", "6495ED"), ("cornsilk", "FFF8DC"), ("crimson", "DC143C"),
    ("cyan", "00FFFF"), ("darkBlue", "00008B"), ("darkCyan", "008B8B"), ("darkGoldenrod", "B8860B"), ("darkGray", "A9A9A9"),
    ("darkGrey", "A9A9A9"), ("darkGreen", "006400"), ("darkKhaki", "BDB76B"), ("darkMagenta", "8B008B"), ("darkOliveGreen", "556B2F"),
    ("darkOrange", "FF8C00"), ("darkOrchid", "9932CC"), ("darkRed", "8B0000"), ("darkSalmon", "E9967A"), ("darkSeaGreen", "8FBC8F"),
    ("darkSlateBlue", "483D8B"), ("darkSlateGray", "2F4F4F"), ("darkSlateGrey", "2F4F4F"), ("darkTurquoise", "00CED1"), ("darkViolet", "9400D3"),
    ("deepPink", "FF1493"), ("deepSkyBlue", "00BFFF"), ("dimGray", "696969"), ("dimGrey", "696969"), ("dodgerBlue", "1E90FF"),
    ("firebrick", "B22222"), ("floralWhite", "FFFAF0"), ("forestGreen", "228B22"), ("fuchsia", "FF00FF"), ("gainsboro", "DCDCDC"),
    ("ghostWhite", "F8F8FF"), ("gold", "FFD700"), ("goldenrod", "DAA520"), ("gray", "808080"), ("grey", "808080"),
    ("green", "008000"), ("greenYellow", "ADFF2F"), ("honeydew", "F0FFF0"), ("hotPink", "FF69B4"), ("indianRed", "CD5C5C"),
    ("indigo", "4B0082"), ("ivory", "FFFFF0"), ("khaki", "F0E68C"), ("lavender", "E6E6FA"), ("lavenderBlush", "FFF0F5"),
    ("lawnGreen", "7CFC00"), ("lemonChiffon", "FFFACD"), ("lightBlue", "ADD8E6"), ("lightCoral", "F08080"), ("lightCyan", "E0FFFF"),
    ("lightGoldenrodYellow", "FAFAD2"), ("lightGray", "D3D3D3"), ("lightGrey", "D3D3D3"), ("lightGreen", "90EE90"), ("lightPink", "FFB6C1"),
    ("lightSalmon", "FFA07A"), ("lightSeaGreen", "20B2AA"), ("lightSkyBlue", "87CEFA"), ("lightSlateGray", "778899"), ("lightSlateGrey", "778899"),
    ("lightSteelBlue", "B0C4DE"), ("lightYellow", "FFFFE0"), ("lime", "00FF00"), ("limeGreen", "32CD32"), ("linen", "FAF0E6"),
    ("magenta", "FF00FF"), ("maroon", "800000"), ("medAquamarine", "66CDAA"), ("medBlue", "0000CD"), ("medOrchid", "BA55D3"),
    ("medPurple", "9370DB"), ("medSeaGreen", "3CB371"), ("medSlateBlue", "7B68EE"), ("medSpringGreen", "00FA9A"), ("medTurquoise", "48D1CC"),
    ("medVioletRed", "C71585"), ("midnightBlue", "191970"), ("mintCream", "F5FFFA"), ("mistyRose", "FFE4E1"), ("moccasin", "FFE4B5"),
    ("navajoWhite", "FFDEAD"), ("navy", "000080"), ("oldLace", "FDF5E6"), ("olive", "808000"), ("oliveDrab", "6B8E23"),
    ("orange", "FFA500"), ("orangeRed", "FF4500"), ("orchid", "DA70D6"), ("paleGoldenrod", "EEE8AA"), ("paleGreen", "98FB98"),
    ("paleTurquoise", "AFEEEE"), ("paleVioletRed", "DB7093"), ("papayaWhip", "FFEFD5"), ("peachPuff", "FFDAB9"), ("peru", "CD853F"),
    ("pink", "FFC0CB"), ("plum", "DDA0DD"), ("powderBlue", "B0E0E6"), ("purple", "800080"), ("red", "FF0000"),
    ("rosyBrown", "BC8F8F"), ("royalBlue", "4169E1"), ("saddleBrown", "8B4513"), ("salmon", "FA8072"), ("sandyBrown", "F4A460"),
    ("seaGreen", "2E8B57"), ("seaShell", "FFF5EE"), ("sienna", "A0522D"), ("silver", "C0C0C0"), ("skyBlue", "87CEEB"),
    ("slateBlue", "6A5ACD"), ("slateGray", "708090"), ("slateGrey", "708090"), ("snow", "FFFAFA"), ("springGreen", "00FF7F"),
    ("steelBlue", "4682B4"), ("tan", "D2B48C"), ("teal", "008080"), ("thistle", "D8BFD8"), ("tomato", "FF6347"),
    ("turquoise", "40E0D0"), ("violet", "EE82EE"), ("wheat", "F5DEB3"), ("white", "FFFFFF"), ("whiteSmoke", "F5F5F5"),
    ("yellow", "FFFF00"), ("yellowGreen", "9ACD32"),
];

// ---------------------------------------------------------------- writing

/// The order of `a:rPr` children (CT_TextCharacterProperties).
const RPR_ORDER: &[&str] = &[
    "ln",
    "noFill",
    "solidFill",
    "gradFill",
    "blipFill",
    "pattFill",
    "grpFill",
    "effectLst",
    "effectDag",
    "highlight",
    "uLnTx",
    "uLn",
    "uFillTx",
    "uFill",
    "latin",
    "ea",
    "cs",
    "sym",
    "hlinkClick",
    "hlinkMouseOver",
    "rtl",
    "extLst",
];

/// Make `rpr` show `want` where it differs from `had` (what the text showed
/// for the run): a value equal to what the run inherits (`base`) removes the
/// run's own, any other is set on the run. A key `want` does not state is
/// left as it is.
pub fn write(rpr: &mut Element, want: &TextStyle, had: &TextStyle, base: &RunStyle) -> Result<(), String> {
    let inherited = base.shown();
    if let Some(s) = want.size.filter(|s| Some(*s) != had.size) {
        if Some(s) == inherited.size {
            rpr.remove_attr("sz");
        } else {
            rpr.set("sz", &s.to_string());
        }
    }
    if let Some(c) = want.color.as_ref().filter(|c| Some(*c) != had.color.as_ref()) {
        rpr.children.retain(|n| !matches!(n, Node::El(e) if FILLS.contains(&e.name.as_str())));
        if Some(c) != inherited.color.as_ref() {
            if is_kept(c) {
                return Err(format!(
                    "color={c} is shown for a colour the file stores with an adjustment, a gradient, a pattern or a picture, and cannot be written: write a colour, {}",
                    style::COLOR_FORM
                ));
            }
            insert_ordered(rpr, fragment(&fill_xml(c)), RPR_ORDER);
        }
    }
    if let Some(f) = want.font.as_ref().filter(|f| Some(*f) != had.font.as_ref()) {
        rpr.children.retain(|n| !matches!(n, Node::El(e) if e.is("a:latin") || e.is("a:ea")));
        if Some(f) != inherited.font.as_ref() {
            insert_ordered(rpr, Element::new("a:latin").with_attr("typeface", f), RPR_ORDER);
            // Where the run shows its East Asian font, that is the one written too.
            if base.ea.is_some() {
                insert_ordered(rpr, Element::new("a:ea").with_attr("typeface", f), RPR_ORDER);
            }
        }
    }
    Ok(())
}
